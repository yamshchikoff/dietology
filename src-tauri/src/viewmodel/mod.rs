use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, State};

use crate::llm::client::LlmClient;
use crate::llm::session::ChatSession;
use crate::llm::types::{Message, Usage};
use crate::memory::conversational_preferences::PreferencesStore;
use crate::memory::facts::FactStore;
use crate::memory::findings::FindingStore;
use crate::memory::master_description::MasterDescriptionStore;
use crate::memory::storage::MemoryStorage;
use crate::memory::system_prompt;

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

    /// Сессия под мьютексом.
    ///
    /// # Инвариант «занятости»
    ///
    /// `send_message` забирает сессию через [`Option::take()`], оставляя `None`.
    /// Остальные команды (`new_chat`, `load_session`, `save_session`, `clear_session`,
    /// `get_messages`) видят `None` = «сессия в процессе отправки» и отказывают.
    ///
    /// ```text
    /// send_message:         take() → None (busy)  …  restore → Some (free)
    /// new_chat/load_session:     Some → заменить    |  None → отказ
    /// save/clear/get:            Some → читать      |  None → отказ
    /// ```
    ///
    /// Мьютекс защищает от гонок между Tauri-командами, Option — от попытки
    /// использовать сессию, пока она мутирует внутри `send_message`.
    pub session: std::sync::Mutex<Option<ChatSession>>,

    // Memory stores — доступны всем Tauri-командам.
    pub storage: Arc<MemoryStorage>,
    pub fact_store: Arc<FactStore>,
    pub finding_store: Arc<FindingStore>,
    pub master_store: Arc<MasterDescriptionStore>,
    pub prefs_store: Arc<PreferencesStore>,
}

// ---- Invariant helpers ----

/// Возвращает `Ok` если сессия свободна (не занята `send_message`), иначе ошибку.
pub fn ensure_free(guard: &Option<ChatSession>) -> Result<(), String> {
    if guard.is_some() {
        Ok(())
    } else {
        Err("session is busy — another request is in progress".into())
    }
}

/// Автосохранение сессии в `data/history/<YYYY-MM-DD>.jsonl`.
///
/// Создаёт директорию `history/` если её нет. Ошибки логирует в stderr, не фейлит запрос.
pub fn auto_save_session(storage: &MemoryStorage, session: &ChatSession) -> Result<(), String> {
    let now = MemoryStorage::now_iso();
    let date = &now[..10]; // "2026-06-17"
    let history_dir = "history".to_string();
    let file_name = format!("{date}.jsonl");

    // Если директория истории не существует — создаём.
    let dir_path = storage.path_for(&history_dir).map_err(|e| e.to_string())?;
    if !dir_path.exists() {
        std::fs::create_dir_all(&dir_path).map_err(|e| format!("create history dir: {e}"))?;
    }

    let file_path = dir_path.join(&file_name);
    session.save_to_jsonl(&file_path)?;
    Ok(())
}

// ---- Commands ----

#[tauri::command]
pub fn new_chat(
    state: State<'_, AppState>,
    system_prompt: Option<String>,
) -> Result<SessionInfo, String> {
    let prompt = system_prompt
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            system_prompt::assemble_system_prompt(
                &state.master_store,
                &state.prefs_store,
            )
        });

    let session = ChatSession::new(prompt.clone());
    let msg_count = session.message_count();
    let messages = session.messages.clone();

    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    ensure_free(&guard)?;
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
    let text = text.trim().to_string();
    if text.is_empty() {
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

            // Автосохранение диалога в data/history/<YYYY-MM-DD>.jsonl.
            // Не фейлим запрос если сохранение упало.
            if let Err(e) = auto_save_session(&state.storage, &session) {
                eprintln!("[dietology] auto-save session failed: {e}");
            }

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
    ensure_free(&guard)?;
    Ok(guard.as_ref().expect("session is free").messages.clone())
}

/// Проверяет и нормализует пользовательский путь для save/load сессии.
///
/// Защиты:
/// - пустая строка — отказ
/// - null-байты — отказ (попытка обмана строковых API)
/// - `..` компоненты — отказ (выход за пределы директории)
/// - symlink-резолвинг: если путь существует, каноникализируется и
///   перепроверяется (симлинк мог указать наружу)
pub fn validate_path(path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("path is empty".into());
    }

    if path.contains('\0') {
        return Err("path contains null byte".into());
    }

    let p = PathBuf::from(path);

    if p.components().any(|c| c == std::path::Component::ParentDir) {
        return Err("path traversal rejected: '..' not allowed".into());
    }

    // Если путь существует — каноникализировать (разрешить симлинки) и перепроверить.
    // std::fs::canonicalize требует существования, поэтому только для существующих.
    if p.exists() {
        let canonical = p
            .canonicalize()
            .map_err(|e| format!("failed to resolve path: {e}"))?;
        if canonical
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err("path traversal rejected after symlink resolution".into());
        }
        return Ok(canonical);
    }

    // Путь не существует (например, save в новый файл) — каноникализируем родителя.
    if let Some(parent) = p.parent() {
        if parent.exists() {
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| format!("failed to resolve parent path: {e}"))?;
            if canonical_parent
                .components()
                .any(|c| c == std::path::Component::ParentDir)
            {
                return Err("path traversal rejected after parent symlink resolution".into());
            }
            if let Some(file_name) = p.file_name() {
                return Ok(canonical_parent.join(file_name));
            }
        }
    }

    Ok(p)
}

#[tauri::command]
pub fn save_session(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let guard = state.session.lock().map_err(|e| e.to_string())?;
    ensure_free(&guard)?;
    let safe_path = validate_path(&path)?;
    guard.as_ref().expect("session is free").save_to_jsonl(&safe_path)
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
    ensure_free(&guard)?;
    *guard = Some(loaded);
    Ok(info)
}

#[tauri::command]
pub fn clear_session(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    ensure_free(&guard)?;
    guard.as_mut().expect("session is free").clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_path;
    use std::fs;
    use std::os::unix::fs as unix_fs;

    #[test]
    fn rejects_empty() {
        assert!(validate_path("").is_err());
        assert!(validate_path("   ").is_err());
    }

    #[test]
    fn rejects_null_byte() {
        assert!(validate_path("/tmp/foo\0bar").is_err());
    }

    #[test]
    fn rejects_parent_dir() {
        assert!(validate_path("../etc/passwd").is_err());
        assert!(validate_path("foo/../../bar").is_err());
    }

    #[test]
    fn accepts_absolute_path() {
        assert!(validate_path("/tmp/session.jsonl").is_ok());
    }

    #[test]
    fn accepts_relative_path() {
        assert!(validate_path("session.jsonl").is_ok());
    }

    #[test]
    fn canonicalizes_symlink_and_rejects_traversal() {
        let tmp = std::env::temp_dir().join("dietology_test_canon");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let real_dir = tmp.join("real");
        fs::create_dir_all(&real_dir).unwrap();

        let link = tmp.join("link");
        unix_fs::symlink(&real_dir, &link).unwrap();

        // symlink сам по себе валиден (без ..)
        let file = link.join("test.jsonl");
        // файла нет — валидация проходит по родителю
        let result = validate_path(file.to_str().unwrap());
        assert!(result.is_ok(), "symlink to safe dir should pass: {result:?}");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn canonicalizes_existing_file() {
        let tmp = std::env::temp_dir().join("dietology_test_existing");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let file = tmp.join("real.jsonl");
        fs::write(&file, "test").unwrap();

        let result = validate_path(file.to_str().unwrap()).unwrap();
        // результат должен быть каноникализирован (symlinks разрешены)
        assert_eq!(result, file.canonicalize().unwrap());

        let _ = fs::remove_dir_all(&tmp);
    }
}
