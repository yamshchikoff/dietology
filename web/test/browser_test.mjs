// Browser streaming integration test.
// Starts the mock server, drives headless Chromium via Puppeteer,
// and verifies that streaming messages are correctly displayed.
//
// Usage: node web/test/browser_test.mjs [mock_server_port]
// Exit code: 0 on success, 1 on failure.

import puppeteer from 'puppeteer';
import { spawn } from 'node:child_process';
import { setTimeout as sleep } from 'node:timers/promises';

const PORT = process.argv[2] || '8765';
const BASE = `http://localhost:${PORT}`;
const MOCK_SERVER_CMD = 'cargo';
const MOCK_SERVER_ARGS = ['run', '--bin', 'test_mock_server'];

let server;
let browser;
let failures = 0;

function log(level, msg) {
  const ts = new Date().toISOString().slice(11, 19);
  process.stderr.write(`[${ts}] [${level}] ${msg}\n`);
}

// ---- Mock server lifecycle ----

async function startMockServer() {
  return new Promise((resolve, reject) => {
    const env = { ...process.env, PORT, WEB_DIR: '../web' };
    server = spawn(MOCK_SERVER_CMD, MOCK_SERVER_ARGS, {
      cwd: 'src-tauri',
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let started = false;
    const timeout = setTimeout(() => {
      if (!started) reject(new Error('mock server start timeout'));
    }, 30000);

    server.stdout.on('data', (data) => {
      const text = data.toString();
      if (!started && text.includes('Mock LLM server listening')) {
        started = true;
        clearTimeout(timeout);
        log('INFO', 'Mock server started');
        resolve();
      }
    });
    server.stderr.on('data', (data) => {
      process.stderr.write(`[mock-server] ${data}`);
    });
    server.on('error', reject);
  });
}

function stopMockServer() {
  if (server) { server.kill('SIGTERM'); server = null; }
}

// ---- Browser lifecycle ----

async function startBrowser() {
  browser = await puppeteer.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage'],
  });
  log('INFO', 'Browser started');
}

async function stopBrowser() {
  if (browser) { await browser.close(); browser = null; }
}

// ---- Page helpers ----

async function newPage() {
  const page = await browser.newPage();
  page.on('console', (msg) => log('PAGE', msg.text()));
  page.on('pageerror', (err) => log('ERROR', `Page error: ${err.message}`));
  return page;
}

async function connectToApp(page) {
  await page.goto(BASE, { waitUntil: 'networkidle0' });

  // Enter API key (any value works with mock server)
  await page.waitForSelector('#key-input', { timeout: 5000 });
  await page.type('#key-input', 'sk-mock-key-12345');
  await page.click('#connect-btn');

  // Wait for chat screen to appear
  await page.waitForSelector('#chat-screen:not([style*="display: none"])', { timeout: 5000 });
  // Wait for ready status
  await page.waitForFunction(() => {
    const el = document.getElementById('status');
    return el && el.textContent === 'Готов';
  }, { timeout: 5000 });
  log('INFO', 'Connected to mock server');
}

async function sendMessage(page, text) {
  await page.type('#input', text);
  await page.click('#send-btn');
}

async function waitForStreamEnd(page, timeoutMs = 30000) {
  await page.waitForFunction(() => {
    const status = document.getElementById('status');
    return status && status.textContent !== 'Думаю...';
  }, { timeout: timeoutMs });
}

async function getLastAssistantText(page) {
  return page.evaluate(() => {
    const msgs = document.querySelectorAll('#chat .msg.assistant');
    const last = msgs[msgs.length - 1];
    return last ? last.textContent : null;
  });
}

async function getLastMsgClass(page) {
  return page.evaluate(() => {
    const msgs = document.querySelectorAll('#chat .msg');
    const last = msgs[msgs.length - 1];
    return last ? last.className : null;
  });
}

async function getStatusText(page) {
  return page.evaluate(() => document.getElementById('status').textContent);
}

async function clearAndReset(page) {
  // Click "Очистить" button
  const btns = await page.$$('.controls button');
  for (const btn of btns) {
    const text = await page.evaluate(el => el.textContent, btn);
    if (text === 'Очистить') {
      await btn.click();
      break;
    }
  }
  await sleep(500);
  await page.evaluate(async () => {
    const resp = await fetch('/api/new_chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ system_prompt: null }),
    });
    if (resp.ok) {
      document.getElementById('chat').innerHTML = '';
    }
  });
  await sleep(300);
}

// ---- Assertions ----

async function assertContains(haystack, needle, testName) {
  if (!haystack || !haystack.includes(needle)) {
    failures++;
    log('FAIL', `${testName}: expected text to contain "${needle.slice(0, 80)}"`);
    log('FAIL', `  Actual text (first 200 chars): "${(haystack || 'NULL').slice(0, 200)}"`);
  } else {
    log('PASS', testName);
  }
}

async function assertNotContains(haystack, needle, testName) {
  if (haystack && haystack.includes(needle)) {
    failures++;
    log('FAIL', `${testName}: text should NOT contain "${needle}"`);
  } else {
    log('PASS', testName);
  }
}

async function assertClassContains(className, expected, testName) {
  if (!className || !className.includes(expected)) {
    failures++;
    log('FAIL', `${testName}: expected class to contain "${expected}", got "${className}"`);
  } else {
    log('PASS', testName);
  }
}

// ---- Test scenarios ----

async function testLongMessageWithTools(page) {
  log('INFO', '=== TEST: Long message with 5 tool calls ===');
  await clearAndReset(page);
  await sendMessage(page, '__test_long_with_tools__');
  await waitForStreamEnd(page, 30000);

  const text = await getLastAssistantText(page);
  const className = await getLastMsgClass(page);
  const status = await getStatusText(page);

  // Should contain all tool annotations
  await assertContains(text, '[tool: describe_dri_minerals...]', 'Long: tool 1 annotation');
  await assertContains(text, '[tool: query_dri_vitamins...]', 'Long: tool 2 annotation');
  await assertContains(text, '[tool: query_dri_minerals...]', 'Long: tool 3 annotation');
  await assertContains(text, '[tool: query_usda_foods...]', 'Long: tool 4 annotation');
  await assertContains(text, '[tool: query_who_hb...]', 'Long: tool 5 annotation');

  // Should contain tokens from different sections
  await assertContains(text, 'token_1', 'Long: early token present');
  await assertContains(text, 'token_40', 'Long: pre-tool token present');
  await assertContains(text, 'after_tool1_1', 'Long: mid-tool token present');
  await assertContains(text, 'after_tool2_25', 'Long: mid-tool2 token present');
  await assertContains(text, 'after_tool3_50', 'Long: mid-tool3 token present');
  await assertContains(text, 'after_tool4_30', 'Long: mid-tool4 token present');
  await assertContains(text, 'final_1', 'Long: final token present');
  await assertContains(text, 'final_30', 'Long: last final token present');

  // Should NOT be an error
  await assertNotContains(text, 'Ошибка', 'Long: no error in text');
  await assertClassContains(className, 'assistant', 'Long: class is assistant');

  // Status should show token counts
  await assertContains(status, 'Токенов', 'Long: status shows token count');

  // Verify total length — should be substantial
  const len = text ? text.length : 0;
  log('INFO', `Long message total length: ${len} chars`);
  if (len < 500) {
    failures++;
    log('FAIL', `Long: message too short (${len} chars), expected > 500`);
  } else {
    log('PASS', 'Long: message has substantial length');
  }
}

async function testErrorMidStream(page) {
  log('INFO', '=== TEST: Error mid-stream preserves partial text ===');
  await clearAndReset(page);
  await sendMessage(page, '__test_error_mid_stream__');
  await waitForStreamEnd(page, 15000);

  const text = await getLastAssistantText(page);
  const className = await getLastMsgClass(page);
  const status = await getStatusText(page);

  // Should contain partial tokens before error
  await assertContains(text, 'token_before_error_1', 'Error: first token present');
  await assertContains(text, 'token_before_error_30', 'Error: middle token present');
  await assertContains(text, 'token_before_error_60', 'Error: last token before error present');

  // Should append error diagnostic
  await assertContains(text, 'Ошибка', 'Error: error indicator present');
  await assertContains(text, 'API error 500', 'Error: error message present');

  // Should NOT have error class (we removed it from the fix)
  await assertNotContains(className, 'error', 'Error: no error CSS class on message');

  // Status should show error
  await assertContains(status, 'Ошибка', 'Error: status shows error');
}

async function testToolDiagnostic(page) {
  log('INFO', '=== TEST: Tool returns error, model continues with diagnostic ===');
  await clearAndReset(page);
  await sendMessage(page, '__test_tool_diagnostic__');
  await waitForStreamEnd(page, 15000);

  const text = await getLastAssistantText(page);
  const className = await getLastMsgClass(page);

  // Should contain the intro text
  await assertContains(text, 'Let me check the DRI data', 'Diag: intro preserved');

  // Should contain tool call annotation
  await assertContains(text, '[tool: query_dri_minerals...]', 'Diag: tool call shown');

  // Should contain the diagnostic text that came after tool
  await assertContains(text, 'partial results', 'Diag: diagnostic text preserved');
  await assertContains(text, 'calcium needs', 'Diag: continuation text preserved');
  await assertContains(text, 'reference range for iron', 'Diag: further text preserved');

  // Should NOT be an error
  await assertNotContains(text, 'Ошибка', 'Diag: no error in text');
  await assertClassContains(className, 'assistant', 'Diag: class is assistant');

  log('INFO', `Tool diagnostic message length: ${text ? text.length : 0} chars`);
}

async function testUnicode(page) {
  log('INFO', '=== TEST: Unicode (Russian + emoji) handling ===');
  await clearAndReset(page);
  await sendMessage(page, '__test_unicode__');
  await waitForStreamEnd(page, 15000);

  const text = await getLastAssistantText(page);
  const className = await getLastMsgClass(page);

  // Check Russian text
  await assertContains(text, 'Привет', 'Unicode: Russian greeting');
  await assertContains(text, 'суточная норма', 'Unicode: Russian text');
  await assertContains(text, 'кальция', 'Unicode: Russian word');

  // Check emoji
  await assertContains(text, '\u{1f956}', 'Unicode: avocado emoji');
  await assertContains(text, '\u{1f966}', 'Unicode: broccoli emoji');
  await assertContains(text, '\u{2705}', 'Unicode: checkmark emoji');

  // Check English
  await assertContains(text, 'Iron needs', 'Unicode: English text');

  // No error
  await assertNotContains(text, 'Ошибка', 'Unicode: no error');
  await assertClassContains(className, 'assistant', 'Unicode: class is assistant');
}

async function testManyTools(page) {
  log('INFO', '=== TEST: Many tools (15 tool calls) ===');
  await clearAndReset(page);
  await sendMessage(page, '__test_many_tools__');
  await waitForStreamEnd(page, 30000);

  const text = await getLastAssistantText(page);
  const className = await getLastMsgClass(page);

  // Check all 15 tool calls are present
  const tools = [
    'describe_dri_minerals', 'describe_dri_vitamins', 'describe_dri_per_kg',
    'describe_usda_foods', 'describe_who_hb', 'describe_who_anaemia',
    'describe_who_bmi', 'describe_who_diabetes', 'describe_lab_ranges',
    'query_dri_minerals', 'query_dri_vitamins', 'query_dri_per_kg',
    'query_usda_foods', 'query_who_hb', 'query_who_anaemia',
  ];
  for (const tool of tools) {
    await assertContains(text, `[tool: ${tool}...]`, `ManyTools: ${tool}`);
  }

  // Check final text present
  await assertContains(text, 'All tools completed', 'ManyTools: final text');
  await assertNotContains(text, 'Ошибка', 'ManyTools: no error');
  await assertClassContains(className, 'assistant', 'ManyTools: class is assistant');

  log('INFO', `Many tools message length: ${text ? text.length : 0} chars`);
}

async function testSimpleMessage(page) {
  log('INFO', '=== TEST: Simple message (default scenario) ===');
  await clearAndReset(page);
  await sendMessage(page, 'How much calcium do I need daily?');
  await waitForStreamEnd(page, 10000);

  const text = await getLastAssistantText(page);
  const className = await getLastMsgClass(page);
  const status = await getStatusText(page);

  await assertContains(text, 'Mock response to', 'Simple: mock response present');
  await assertContains(text, 'calcium', 'Simple: original query in response');
  await assertNotContains(text, 'Ошибка', 'Simple: no error');
  await assertClassContains(className, 'assistant', 'Simple: class is assistant');
  await assertContains(status, 'Токенов', 'Simple: status shows token count');
}

// ---- Main ----

async function main() {
  log('INFO', 'Starting browser streaming integration tests');
  log('INFO', `Mock server port: ${PORT}`);

  try {
    await startMockServer();
  } catch (e) {
    log('FATAL', `Failed to start mock server: ${e.message}`);
    process.exit(1);
  }

  try {
    await startBrowser();
  } catch (e) {
    log('FATAL', `Failed to start browser: ${e.message}`);
    stopMockServer();
    process.exit(1);
  }

  try {
    const page = await newPage();

    // Connect once, then run all tests sharing the same page
    await connectToApp(page);

    await testSimpleMessage(page);
    await testLongMessageWithTools(page);
    await testErrorMidStream(page);
    await testToolDiagnostic(page);
    await testUnicode(page);
    await testManyTools(page);

    await page.close();
  } catch (e) {
    log('FATAL', `Test error: ${e.message}`);
    log('FATAL', e.stack);
    failures++;
  } finally {
    await stopBrowser();
    stopMockServer();
  }

  log('INFO', `Tests completed: ${failures} failures`);
  process.exit(failures > 0 ? 1 : 0);
}

main();
