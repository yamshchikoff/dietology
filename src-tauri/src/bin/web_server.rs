use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use axum::{
    extract::State,
    http::StatusCode,
    response::{Json, Sse},
    routing::{get, post},
    Router,
};
use axum::response::sse::{Event, KeepAlive};
use futures::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use dietology_lib::data::DataLoader;
use dietology_lib::llm::client::LlmClient;
use dietology_lib::llm::session::ChatSession;
use dietology_lib::llm::types::{Message, Usage};
use dietology_lib::memory::conversational_preferences::PreferencesStore;
use dietology_lib::memory::facts::FactStore;
use dietology_lib::memory::findings::FindingStore;
use dietology_lib::memory::master_description::{LlmCredentials, MasterDescriptionStore};
use dietology_lib::memory::storage::MemoryStorage;
use dietology_lib::memory::tools;
use dietology_lib::tools::registry::ToolRegistry;
use dietology_lib::tools::{describe, query};
use dietology_lib::viewmodel::{self, SessionInfo};

// ---- App State ----

/// Web-специфичное состояние. LlmClient создаётся, когда пользователь предоставляет ключ.
///
/// Все Mutex — `std::sync::Mutex`, не `tokio::sync::Mutex`.
/// Инвариант: блокировка НИКОГДА не удерживается через `.await`.
/// Нарушение = блокировка tokio worker thread.
struct WebState {
    llm_client: Mutex<Option<Arc<LlmClient>>>,
    session: Mutex<Option<ChatSession>>,
    registry: Arc<ToolRegistry>,
    llm_creds: Arc<Mutex<Option<LlmCredentials>>>,
}

type SharedState = Arc<WebState>;

impl WebState {
    fn require_client(&self) -> Result<Arc<LlmClient>, (StatusCode, String)> {
        self.llm_client
            .lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .clone()
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    "API key not set. Use POST /api/set_key".into(),
                )
            })
    }
}

// ---- Request DTOs ----

#[derive(Deserialize)]
struct SetKeyRequest {
    api_key: String,
    #[serde(default = "default_base_url")]
    api_base_url: String,
    #[serde(default = "default_model")]
    model: String,
}

fn default_base_url() -> String {
    "https://api.deepseek.com/anthropic".into()
}

fn default_model() -> String {
    "deepseek-chat".into()
}

#[derive(Deserialize)]
struct NewChatRequest {
    system_prompt: Option<String>,
}

#[derive(Deserialize)]
struct SendMessageRequest {
    text: String,
}

#[derive(Deserialize)]
struct PathRequest {
    path: String,
}

// ---- Helpers ----

fn map_lock_err(e: impl ToString) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

fn map_busy(e: String) -> (StatusCode, String) {
    (StatusCode::CONFLICT, e)
}

fn ok() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true}))
}

fn sse_event(name: &str, data: serde_json::Value) -> Event {
    Event::default().event(name).data(data.to_string())
}

// ---- Handlers ----

async fn set_key_handler(
    State(state): State<SharedState>,
    Json(body): Json<SetKeyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if body.api_key.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "api_key is empty".into()));
    }

    let llm_client = LlmClient::with_credentials(
        state.registry.clone(),
        body.api_key.clone(),
        body.api_base_url.clone(),
        body.model.clone(),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut guard = state.llm_client.lock().map_err(map_lock_err)?;
    *guard = Some(Arc::new(llm_client));

    let mut creds_guard = state.llm_creds.lock().map_err(map_lock_err)?;
    *creds_guard = Some(LlmCredentials {
        api_key: body.api_key,
        api_base_url: body.api_base_url,
        model: body.model,
    });

    Ok(ok())
}

async fn new_chat_handler(
    State(state): State<SharedState>,
    Json(body): Json<NewChatRequest>,
) -> Result<Json<SessionInfo>, (StatusCode, String)> {
    let _ = state.require_client()?;

    let prompt = body
        .system_prompt
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| viewmodel::DEFAULT_SYSTEM_PROMPT.into());

    let session = ChatSession::new(prompt.clone());
    let info = SessionInfo {
        system_prompt: prompt,
        message_count: session.message_count(),
        messages: session.messages.clone(),
        usage: Usage {
            input_tokens: 0,
            output_tokens: 0,
        },
    };

    let mut guard = state.session.lock().map_err(map_lock_err)?;
    viewmodel::ensure_free(&guard).map_err(map_busy)?;
    *guard = Some(session);

    Ok(Json(info))
}

async fn send_message_handler(
    State(state): State<SharedState>,
    Json(body): Json<SendMessageRequest>,
) -> Result<Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)> {
    let text = body.text.trim().to_string();
    if text.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message text is empty".into()));
    }

    let llm_client = state.require_client()?;

    let mut session = {
        let mut guard = state.session.lock().map_err(map_lock_err)?;
        guard
            .take()
            .ok_or_else(|| (StatusCode::CONFLICT, "session is busy".into()))?
    };

    let len_before = session.messages.len();
    session.add_user_message(text);
    let system_prompt = session.system_prompt.clone();

    let (tx, rx) = mpsc::unbounded_channel::<Event>();

    let state_clone = state.clone();
    tokio::spawn(async move {
        let result = llm_client
            .chat_streaming(
                &mut session.messages,
                &system_prompt,
                |text: &str| {
                    let _ = tx.send(sse_event("token", serde_json::json!({"delta": text})));
                },
                |name: &str| {
                    let _ = tx.send(sse_event("tool_start", serde_json::json!({"name": name})));
                },
                |name: &str| {
                    let _ = tx.send(sse_event("tool_done", serde_json::json!({"name": name})));
                },
            )
            .await;

        match result {
            Ok(response) => {
                session.add_usage(response.usage);
                let _ = tx.send(sse_event(
                    "done",
                    serde_json::json!({
                        "final_text": response.final_text,
                        "usage": {
                            "input_tokens": response.usage.input_tokens,
                            "output_tokens": response.usage.output_tokens,
                        }
                    }),
                ));
                match state_clone.session.lock() {
                    Ok(mut guard) => *guard = Some(session),
                    Err(poison) => {
                        let _ = tx.send(sse_event(
                            "error",
                            serde_json::json!({"message": format!("Internal lock error: {poison}")}),
                        ));
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                session.messages.truncate(len_before);
                let _ = tx.send(sse_event(
                    "error",
                    serde_json::json!({"message": error_msg}),
                ));
                match state_clone.session.lock() {
                    Ok(mut guard) => *guard = Some(session),
                    Err(poison) => {
                        let _ = tx.send(sse_event(
                            "error",
                            serde_json::json!({"message": format!("Internal lock error: {poison}")}),
                        ));
                    }
                }
            }
        }
    });

    let stream = UnboundedReceiverStream::new(rx).map(Ok);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn get_messages_handler(
    State(state): State<SharedState>,
) -> Result<Json<Vec<Message>>, (StatusCode, String)> {
    let _ = state.require_client()?;
    let guard = state.session.lock().map_err(map_lock_err)?;
    viewmodel::ensure_free(&guard).map_err(map_busy)?;
    Ok(Json(
        guard.as_ref().expect("session is free").messages.clone(),
    ))
}

async fn save_session_handler(
    State(state): State<SharedState>,
    Json(body): Json<PathRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let _ = state.require_client()?;
    let guard = state.session.lock().map_err(map_lock_err)?;
    viewmodel::ensure_free(&guard).map_err(map_busy)?;
    let safe_path =
        viewmodel::validate_path(&body.path).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    guard
        .as_ref()
        .expect("session is free")
        .save_to_jsonl(&safe_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(ok())
}

async fn load_session_handler(
    State(state): State<SharedState>,
    Json(body): Json<PathRequest>,
) -> Result<Json<SessionInfo>, (StatusCode, String)> {
    let _ = state.require_client()?;
    let safe_path =
        viewmodel::validate_path(&body.path).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let loaded = ChatSession::load_from_jsonl(&safe_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let info = SessionInfo {
        system_prompt: loaded.system_prompt.clone(),
        message_count: loaded.message_count(),
        messages: loaded.messages.clone(),
        usage: loaded.total_usage,
    };
    let mut guard = state.session.lock().map_err(map_lock_err)?;
    viewmodel::ensure_free(&guard).map_err(map_busy)?;
    *guard = Some(loaded);
    Ok(Json(info))
}

async fn clear_session_handler(
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let _ = state.require_client()?;
    let mut guard = state.session.lock().map_err(map_lock_err)?;
    viewmodel::ensure_free(&guard).map_err(map_busy)?;
    guard.as_mut().expect("session is free").clear();
    Ok(ok())
}

// ---- Main ----

#[tokio::main]
async fn main() {
    let loader = DataLoader::for_development();
    let mut registry = ToolRegistry::new();
    describe::register_describe_tools(&mut registry, &loader);
    query::register_query_tools(&mut registry, &loader);

    let storage = Arc::new(MemoryStorage::for_development());
    let fact_store = Arc::new(FactStore::new(storage.clone()));
    let finding_store = Arc::new(FindingStore::new(storage.clone(), fact_store.clone()));
    let master_store = Arc::new(MasterDescriptionStore::new(storage.clone()));
    let prefs_store = Arc::new(PreferencesStore::new(storage.clone()));

    tools::register_memory_read_tools(
        &mut registry,
        fact_store.clone(),
        finding_store.clone(),
        master_store.clone(),
        prefs_store.clone(),
    );

    let llm_creds: Arc<Mutex<Option<LlmCredentials>>> = Arc::new(Mutex::new(None));
    tools::register_memory_write_tools(
        &mut registry,
        fact_store,
        finding_store,
        master_store,
        prefs_store,
        llm_creds.clone(),
    );

    let state = Arc::new(WebState {
        llm_client: Mutex::new(None),
        session: Mutex::new(Some(
            std::env::var("SESSION_PATH")
                .ok()
                .and_then(|p| {
                    viewmodel::validate_path(&p)
                        .ok()
                        .and_then(|safe_path| ChatSession::load_from_jsonl(&safe_path).ok())
                })
                .unwrap_or_else(|| ChatSession::new(viewmodel::DEFAULT_SYSTEM_PROMPT.into())),
        )),
        registry: Arc::new(registry),
        llm_creds,
    });

    let web_dir = std::env::var("WEB_DIR").unwrap_or_else(|_| "web".into());

    let app = Router::new()
        .route("/api/set_key", post(set_key_handler))
        .route("/api/new_chat", post(new_chat_handler))
        .route("/api/send_message", post(send_message_handler))
        .route("/api/messages", get(get_messages_handler))
        .route("/api/save_session", post(save_session_handler))
        .route("/api/load_session", post(load_session_handler))
        .route("/api/clear_session", post(clear_session_handler))
        .fallback_service(ServeDir::new(&web_dir))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");
    println!("Dietology web server listening on http://{addr}");
    axum::serve(listener, app)
        .await
        .expect("web server failed");
}
