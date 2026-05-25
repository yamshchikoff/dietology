use serde::Serialize;
use std::path::PathBuf;
use tauri::{Emitter, State};

use crate::llm::client::LlmClient;
use crate::llm::session::ChatSession;
use crate::llm::types::{Message, Usage};

// ---- DTOs ----

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub system_prompt: String,
    pub message_count: usize,
    pub messages: Vec<Message>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    pub final_text: String,
    pub visualization_json: Option<serde_json::Value>,
    pub usage: Usage,
}

// ---- App State ----

pub struct AppState {
    pub llm_client: LlmClient,
    /// Инвариант занятости: None = сессия занята (send_message забрал её через take()),
    /// Some = сессия свободна для new_chat / load_session.
    /// Мьютекс защищает от гонок, Option — от одновременного использования сессии.
    pub session: std::sync::Mutex<Option<ChatSession>>,
}

const DEFAULT_SYSTEM_PROMPT: &str = "\
Ты — ассистент по питанию. Отвечай на русском языке.
Для поиска данных используй инструменты: сначала describe для навигации, потом query для конкретных значений.";

// ---- Commands ----

#[tauri::command]
pub fn new_chat(
    state: State<'_, AppState>,
    system_prompt: Option<String>,
) -> Result<SessionInfo, String> {
    let prompt = system_prompt
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.into());

    let session = ChatSession::new(prompt.clone());
    let msg_count = session.message_count();
    let messages = session.messages.clone();

    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        return Err("session is busy — another request is in progress".into());
    }
    *guard = Some(session);

    Ok(SessionInfo {
        system_prompt: prompt,
        message_count: msg_count,
        messages,
        usage: Usage {
            input_tokens: 0,
            output_tokens: 0,
        },
    })
}

#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    text: String,
) -> Result<ChatResponse, String> {
    if text.trim().is_empty() {
        return Err("message text is empty".into());
    }

    let mut session = {
        let mut guard = state.session.lock().map_err(|e| e.to_string())?;
        guard.take().ok_or_else(|| "session is busy — another request is in progress".to_string())?
    };

    let len_before = session.messages.len();
    session.add_user_message(text);
    let system_prompt = session.system_prompt.clone();

    let app_token = app.clone();
    let app_tool_start = app.clone();
    let app_tool_done = app.clone();

    let result = state
        .llm_client
        .chat_streaming(
            &mut session.messages,
            &system_prompt,
            move |text: &str| {
                let _ = app_token.emit("chat:token", serde_json::json!({"delta": text}));
            },
            move |name: &str| {
                let _ = app_tool_start.emit("chat:tool_start", serde_json::json!({"name": name}));
            },
            move |name: &str| {
                let _ = app_tool_done.emit("chat:tool_done", serde_json::json!({"name": name}));
            },
        )
        .await;

    match result {
        Ok(response) => {
            session.add_usage(response.usage);
            let resp = ChatResponse {
                final_text: response.final_text.clone(),
                visualization_json: response.visualization_json,
                usage: response.usage,
            };
            let _ = app.emit(
                "chat:done",
                serde_json::json!({
                    "final_text": response.final_text,
                    "usage": {
                        "input_tokens": response.usage.input_tokens,
                        "output_tokens": response.usage.output_tokens,
                    }
                }),
            );
            let mut guard = state.session.lock().map_err(|e| e.to_string())?;
            *guard = Some(session);
            Ok(resp)
        }
        Err(e) => {
            let error_msg = e.to_string();
            session.messages.truncate(len_before);
            let _ = app.emit("chat:error", serde_json::json!({"message": error_msg}));
            let mut guard = state.session.lock().map_err(|e| e.to_string())?;
            *guard = Some(session);
            Err(error_msg)
        }
    }
}

#[tauri::command]
pub fn get_messages(state: State<'_, AppState>) -> Result<Vec<Message>, String> {
    let guard = state.session.lock().map_err(|e| e.to_string())?;
    let session = guard.as_ref().ok_or_else(|| "session is busy — another request is in progress".to_string())?;
    Ok(session.messages.clone())
}

fn validate_path(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    if p.components().any(|c| c == std::path::Component::ParentDir) {
        return Err("path traversal rejected: '..' not allowed".into());
    }
    Ok(p)
}

#[tauri::command]
pub fn save_session(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let guard = state.session.lock().map_err(|e| e.to_string())?;
    let session = guard.as_ref().ok_or_else(|| "session is busy — another request is in progress".to_string())?;
    let safe_path = validate_path(&path)?;
    session.save_to_jsonl(&safe_path)
}

#[tauri::command]
pub fn load_session(
    state: State<'_, AppState>,
    path: String,
) -> Result<SessionInfo, String> {
    let safe_path = validate_path(&path)?;
    let loaded = ChatSession::load_from_jsonl(&safe_path)?;
    let info = SessionInfo {
        system_prompt: loaded.system_prompt.clone(),
        message_count: loaded.message_count(),
        messages: loaded.messages.clone(),
        usage: loaded.total_usage,
    };
    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        return Err("session is busy — another request is in progress".into());
    }
    *guard = Some(loaded);
    Ok(info)
}

#[tauri::command]
pub fn clear_session(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    let session = guard.as_mut().ok_or_else(|| "session is busy — another request is in progress".to_string())?;
    session.clear();
    Ok(())
}
