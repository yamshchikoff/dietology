use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

use crate::llm::client::LlmClient;
use crate::llm::session::ChatSession;
use crate::llm::types::{Message, Usage};

// ---- DTOs ----

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub system_prompt: String,
    pub message_count: usize,
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
    pub loader: crate::data::DataLoader,
    pub llm_client: LlmClient,
    pub session: std::sync::Mutex<ChatSession>,
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

    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    *session = ChatSession::new(prompt.clone());

    Ok(SessionInfo {
        system_prompt: prompt,
        message_count: 0,
        usage: Usage {
            input_tokens: 0,
            output_tokens: 0,
        },
    })
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    text: String,
) -> Result<ChatResponse, String> {
    if text.trim().is_empty() {
        return Err("message text is empty".into());
    }

    let (mut messages, system_prompt) = {
        let mut session = state.session.lock().map_err(|e| e.to_string())?;
        session.add_user_message(text);
        (session.messages.clone(), session.system_prompt.clone())
    };

    let response = state
        .llm_client
        .chat(&mut messages, &system_prompt)
        .await
        .map_err(|e| e.to_string())?;

    {
        let mut session = state.session.lock().map_err(|e| e.to_string())?;
        session.messages = messages;
        session.add_usage(response.usage);
    }

    Ok(ChatResponse {
        final_text: response.final_text,
        visualization_json: response.visualization_json,
        usage: response.usage,
    })
}

#[tauri::command]
pub fn get_messages(state: State<'_, AppState>) -> Result<Vec<Message>, String> {
    let session = state.session.lock().map_err(|e| e.to_string())?;
    Ok(session.messages.clone())
}

#[tauri::command]
pub fn save_session(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let session = state.session.lock().map_err(|e| e.to_string())?;
    session.save_to_jsonl(&PathBuf::from(path))
}

#[tauri::command]
pub fn load_session(
    state: State<'_, AppState>,
    path: String,
) -> Result<SessionInfo, String> {
    let loaded = ChatSession::load_from_jsonl(&PathBuf::from(path))?;
    let info = SessionInfo {
        system_prompt: loaded.system_prompt.clone(),
        message_count: loaded.message_count(),
        usage: loaded.total_usage,
    };
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    *session = loaded;
    Ok(info)
}

#[tauri::command]
pub fn clear_session(state: State<'_, AppState>) -> Result<(), String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    session.clear();
    Ok(())
}
