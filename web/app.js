let currentMsgElem = null;
let isStreaming = false;

// ---- Cookie helpers (единственное, что хранится в cookie) ----

const COOKIE_KEY = 'dietology_api_key';

function getApiKey() {
  const m = document.cookie.match('(?:^|;)\\s*' + COOKIE_KEY + '=([^;]*)');
  return m ? decodeURIComponent(m[1]) : null;
}

function setApiKey(key) {
  const secure = location.protocol === 'https:' ? ';Secure' : '';
  document.cookie = COOKIE_KEY + '=' + encodeURIComponent(key) + ';path=/;max-age=31536000;SameSite=Strict' + secure;
}

function clearApiKey() {
  document.cookie = COOKIE_KEY + '=;path=/;max-age=0';
}

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
    setApiKey(key);
    await initChat();
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
    if (Array.isArray(info.messages) && info.messages.length > 0) {
      renderMessages(info.messages);
    } else {
      addMsg('system', 'Новая сессия. Задайте вопрос о питании.');
    }
    setStatus('Ready');
  } catch (e) {
    addMsg('error', 'Ошибка инициализации: ' + (e.message || String(e)));
    setStatus('Init error');
  }
}

// ---- SSE streaming ----

function handleSSEvent(name, payload) {
  switch (name) {
    case 'token':
      if (currentMsgElem) {
        currentMsgElem.textContent += payload.delta;
        scrollChat();
      }
      break;
    case 'tool_start':
      if (currentMsgElem) {
        currentMsgElem.textContent += '\n[tool: ' + payload.name + '...]\n';
        scrollChat();
      }
      break;
    case 'tool_done':
      break;
    case 'done':
      if (currentMsgElem) {
        currentMsgElem.textContent = payload.final_text;
      }
      setStatus('Tokens: ' + payload.usage.input_tokens + ' in + ' + payload.usage.output_tokens + ' out');
      resetUI();
      break;
    case 'error':
      if (currentMsgElem) {
        currentMsgElem.className = 'msg error';
        currentMsgElem.textContent = 'Ошибка: ' + payload.message;
      } else {
        addMsg('error', 'Ошибка: ' + payload.message);
      }
      setStatus('Error');
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

      buffer += decoder.decode(value, { stream: true });

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
            // ignore parse errors (keepalive comments)
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
  setStatus('Thinking...');
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
      setStatus('Error');
      resetUI();
    }
  }
}

// ---- Session commands ----

async function saveSession() {
  const path = prompt('File path to save:', '/tmp/dietology_session.jsonl');
  if (!path) return;
  try {
    const resp = await fetch('/api/save_session', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path })
    });
    if (!resp.ok) throw new Error(await resp.text());
    setStatus('Saved to ' + path);
  } catch (e) {
    addMsg('error', 'Ошибка сохранения: ' + (e.message || String(e)));
  }
}

async function loadSession() {
  const path = prompt('File path to load:', '/tmp/dietology_session.jsonl');
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
    setStatus('Loaded ' + info.message_count + ' messages | ' + info.system_prompt.slice(0, 60) + '...');
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
    setStatus('Ready');
  } catch (e) {
    addMsg('error', 'Ошибка очистки: ' + (e.message || String(e)));
  }
}

// ---- Init ----

(async () => {
  const savedKey = getApiKey();
  if (savedKey) {
    // Try to re-initialize server with saved key
    try {
      const resp = await fetch('/api/set_key', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ api_key: savedKey })
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
