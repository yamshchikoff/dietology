let currentMsgElem = null;
let isStreaming = false;

// ---- Cookie helpers ----

const COOKIE_KEY = 'dietology_api_key';
const COOKIE_BASE_URL = 'dietology_base_url';

function getCookie(name) {
  const m = document.cookie.match('(?:^|;)\\s*' + name + '=([^;]*)');
  return m ? decodeURIComponent(m[1]) : null;
}

function setCookie(name, value) {
  const secure = location.protocol === 'https:' ? ';Secure' : '';
  document.cookie = name + '=' + encodeURIComponent(value) + ';path=/;max-age=31536000;SameSite=Strict' + secure;
}

function clearCookie(name) {
  document.cookie = name + '=;path=/;max-age=0';
}

function getApiKey() { return getCookie(COOKIE_KEY); }
function setApiKey(key) { setCookie(COOKIE_KEY, key); }
function clearApiKey() { clearCookie(COOKIE_KEY); }

function getBaseUrl() { return getCookie(COOKIE_BASE_URL); }
function setBaseUrl(url) { setCookie(COOKIE_BASE_URL, url); }

// ---- DOM helpers ----

function addMsg(role, text) {
  const d = document.createElement('div');
  d.className = 'msg ' + role;
  d.textContent = text;
  document.getElementById('chat').appendChild(d);
  scrollChat();
}

function scrollChat() {
  const c = document.getElementById('chat');
  c.scrollTop = c.scrollHeight;
}

function renderMessages(messages) {
  for (const m of messages) {
    if (!Array.isArray(m.content)) continue;
    for (const b of m.content) {
      if (b.type === 'text') addMsg(m.role, b.text);
      else if (b.type === 'tool_use') addMsg('system', '[tool: ' + b.name + ']');
      else if (b.type === 'tool_result') addMsg('system', '[tool result]');
    }
  }
}

function setStatus(text) {
  document.getElementById('status').textContent = text;
}

function resetUI() {
  isStreaming = false;
  currentMsgElem = null;
  document.getElementById('send-btn').disabled = false;
  document.getElementById('input').focus();
}

// ---- Connect ----

async function connect() {
  const key = document.getElementById('key-input').value.trim();
  const baseUrl = document.getElementById('base-url-input').value.trim();
  const errEl = document.getElementById('key-error');

  if (!key) {
    errEl.textContent = 'Введите ключ';
    return;
  }

  document.getElementById('connect-btn').disabled = true;
  errEl.textContent = '';

  try {
    const resp = await fetch('/api/set_key', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ api_key: key, api_base_url: baseUrl })
    });
    if (!resp.ok) {
      const err = await resp.text();
      throw new Error(err);
    }
    const result = await initChat();
    if (!result.ok) {
      errEl.textContent = result.error;
      document.getElementById('connect-btn').disabled = false;
      return;
    }
    setApiKey(key);
    if (baseUrl) setBaseUrl(baseUrl);
    document.getElementById('key-screen').style.display = 'none';
    document.getElementById('chat-screen').style.display = '';
  } catch (e) {
    errEl.textContent = 'Ошибка: ' + (e.message || String(e));
    document.getElementById('connect-btn').disabled = false;
  }
}

async function initChat() {
  try {
    const resp = await fetch('/api/new_chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ system_prompt: null })
    });
    if (!resp.ok) throw new Error(await resp.text());
    const info = await resp.json();
    document.getElementById('chat').innerHTML = '';
    if (Array.isArray(info.messages) && info.messages.length > 0) {
      renderMessages(info.messages);
    } else {
      addMsg('system', 'Новая сессия. Задайте вопрос о питании.');
    }
    setStatus('Готов');
    return { ok: true };
  } catch (e) {
    const errMsg = 'Ошибка инициализации: ' + (e.message || String(e));
    addMsg('error', errMsg);
    setStatus('Ошибка инициализации');
    return { ok: false, error: errMsg };
  }
}

// ---- SSE streaming ----

function handleSSEvent(name, payload) {
  switch (name) {
    case 'token':
      if (currentMsgElem) {
        currentMsgElem.textContent += payload.delta ?? '';
        scrollChat();
      }
      break;
    case 'tool_start':
      if (currentMsgElem) {
        currentMsgElem.textContent += '\n[tool: ' + (payload.name ?? '?') + '...]\n';
        scrollChat();
      }
      break;
    case 'tool_done':
      break;
    case 'done':
      if (currentMsgElem) {
        currentMsgElem.textContent = payload.final_text ?? currentMsgElem.textContent;
      }
      setStatus('Токенов: ' + (payload.usage?.input_tokens ?? '?') + ' вх + ' + (payload.usage?.output_tokens ?? '?') + ' вых');
      resetUI();
      break;
    case 'error':
      if (currentMsgElem) {
        currentMsgElem.className = 'msg error';
        currentMsgElem.textContent = 'Ошибка: ' + payload.message;
      } else {
        addMsg('error', 'Ошибка: ' + payload.message);
      }
      setStatus('Ошибка');
      resetUI();
      break;
  }
}

async function sendMessageSSE(text) {
  const controller = new AbortController();
  let timeout = setTimeout(() => controller.abort(), 120_000);

  const resetTimeout = () => {
    clearTimeout(timeout);
    timeout = setTimeout(() => controller.abort(), 120_000);
  };

  try {
    const response = await fetch('/api/send_message', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ text }),
      signal: controller.signal
    });

    if (!response.ok) {
      const err = await response.text();
      throw new Error(err);
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      resetTimeout();

      buffer += decoder.decode(value, { stream: true }).replace(/\r\n/g, '\n');

      let idx;
      while ((idx = buffer.indexOf('\n\n')) !== -1) {
        const eventText = buffer.slice(0, idx);
        buffer = buffer.slice(idx + 2);

        if (eventText.trim() === '') continue;

        let eventName = '';
        let data = '';
        for (const line of eventText.split('\n')) {
          if (line.startsWith('event: ')) {
            eventName = line.slice(7).trim();
          } else if (line.startsWith('data: ')) {
            // Сервер шлёт single-line JSON — перезапись (а не конкатенация) корректна.
            data = line.slice(6);
          }
        }

        if (eventName && data) {
          try {
            handleSSEvent(eventName, JSON.parse(data));
          } catch (_) {
            // Skip SSE comments/keepalive (no valid JSON payload).
            // Only warn for data that looks like it was meant to be JSON.
            const t = data.trim();
            if (t.startsWith('{') || t.startsWith('[')) {
              console.warn('SSE: failed to parse event data', eventName, t.slice(0, 80));
            }
          }
        }
      }
    }
  } catch (e) {
    if (e.name === 'AbortError') {
      throw new Error('Таймаут запроса: сервер не отвечает');
    }
    throw e;
  } finally {
    clearTimeout(timeout);
  }
}

// ---- send ----

async function send() {
  const input = document.getElementById('input');
  const btn = document.getElementById('send-btn');
  const text = input.value.trim();
  if (!text || isStreaming) return;

  input.value = '';
  btn.disabled = true;
  isStreaming = true;
  setStatus('Думаю...');
  addMsg('user', text);

  const d = document.createElement('div');
  d.className = 'msg assistant';
  document.getElementById('chat').appendChild(d);
  scrollChat();
  currentMsgElem = d;

  try {
    await sendMessageSSE(text);
  } catch (e) {
    if (isStreaming) {
      const errMsg = e.message || String(e);
      currentMsgElem.className = 'msg error';
      currentMsgElem.textContent = 'Ошибка: ' + errMsg;
      setStatus('Ошибка');
      resetUI();
    }
  }
  if (isStreaming) {
    // SSE stream ended without a terminal event (done/error)
    currentMsgElem.className = 'msg error';
    currentMsgElem.textContent = 'Ошибка: неожиданный конец потока';
    setStatus('Ошибка');
    resetUI();
  }
}

// ---- Session commands ----

async function saveSession() {
  const path = prompt('Путь для сохранения:', '/tmp/dietology_session.jsonl');
  if (!path) return;
  try {
    const resp = await fetch('/api/save_session', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path })
    });
    if (!resp.ok) throw new Error(await resp.text());
    setStatus('Сохранено в ' + path);
  } catch (e) {
    addMsg('error', 'Ошибка сохранения: ' + (e.message || String(e)));
  }
}

async function loadSession() {
  const path = prompt('Путь для загрузки:', '/tmp/dietology_session.jsonl');
  if (!path) return;
  try {
    const resp = await fetch('/api/load_session', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path })
    });
    if (!resp.ok) throw new Error(await resp.text());
    const info = await resp.json();
    document.getElementById('chat').innerHTML = '';
    renderMessages(info.messages);
    const promptPreview = (info.system_prompt || '').slice(0, 60);
    setStatus('Загружено ' + info.message_count + ' сообщений | ' + promptPreview + (promptPreview.length >= 60 ? '...' : ''));
  } catch (e) {
    addMsg('error', 'Ошибка загрузки: ' + (e.message || String(e)));
  }
}

async function clearSession() {
  try {
    const resp = await fetch('/api/clear_session', { method: 'POST' });
    if (!resp.ok) throw new Error(await resp.text());
    document.getElementById('chat').innerHTML = '';
    addMsg('system', 'Сессия очищена.');
    setStatus('Готов');
  } catch (e) {
    addMsg('error', 'Ошибка очистки: ' + (e.message || String(e)));
  }
}

// ---- Init ----

(async () => {
  const savedKey = getApiKey();
  const savedBaseUrl = getBaseUrl();
  if (savedBaseUrl) {
    document.getElementById('base-url-input').value = savedBaseUrl;
  }
  if (savedKey) {
    // Try to re-initialize server with saved key
    try {
      const resp = await fetch('/api/set_key', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ api_key: savedKey, api_base_url: savedBaseUrl || undefined })
      });
      if (resp.ok) {
        document.getElementById('key-screen').style.display = 'none';
        document.getElementById('chat-screen').style.display = '';
        await initChat();
        return;
      }
    } catch (_) {
      // Fall through to key screen
    }
    clearApiKey();
  }
  // Show key screen
  document.getElementById('key-input').focus();
})();
