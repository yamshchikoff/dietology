// Mock LLM server for browser streaming tests.
// Serves web/ statically and implements /api/* endpoints identical to web_server,
// but sends synthetic SSE events instead of calling a real LLM.
//
// Usage:
//   cargo run --bin test_mock_server [--port PORT] [--web-dir PATH]
//   Default: PORT=8765, web-dir=web/

use axum::{
    extract::State,
    http::StatusCode,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::stream::{self};
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tower_http::{cors::CorsLayer, services::ServeDir};

#[derive(Debug, Clone, Default)]
struct AppState {
    session: Arc<Mutex<MockSession>>,
}

#[derive(Debug, Clone, Default)]
struct MockSession {
    messages: Vec<MockMessage>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct MockMessage {
    pub role: String,
    pub content: Vec<MockContentBlock>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
enum MockContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Deserialize)]
struct SendMessageRequest {
    text: String,
}

// ---- SSE event helpers ----

fn sse_event(name: &str, data: serde_json::Value) -> Event {
    Event::default().event(name).data(data.to_string())
}

fn json_ok() -> serde_json::Value {
    serde_json::json!({"ok": true})
}

// ---- Scenario generators ----

/// Returns a stream of SSE events for the given message text.
/// The text determines which scenario to run.
fn build_scenario(text: &str) -> Vec<Result<Event, Infallible>> {
    if text.contains("__test_long_with_tools__") {
        scenario_long_with_tools()
    } else if text.contains("__test_error_mid_stream__") {
        scenario_error_mid_stream()
    } else if text.contains("__test_tool_diagnostic__") {
        scenario_tool_diagnostic()
    } else if text.contains("__test_unicode__") {
        scenario_unicode()
    } else if text.contains("__test_many_tools__") {
        scenario_many_tools()
    } else {
        scenario_simple(text)
    }
}

/// 250+ tokens with 5 tool calls interspersed.
/// The final_text in "done" must match what the streaming tokens built.
fn scenario_long_with_tools() -> Vec<Result<Event, Infallible>> {
    let mut events: Vec<Result<Event, Infallible>> = Vec::new();
    let mut accumulated = String::new();

    // Pre-tool tokens
    for i in 1..=40 {
        let t = format!("token_{} ", i);
        accumulated.push_str(&t);
        events.push(Ok(sse_event("token", serde_json::json!({"delta": t}))));
    }

    // Tool 1
    events.push(Ok(sse_event("tool_start", serde_json::json!({"name": "describe_dri_minerals"}))));
    accumulated.push_str("\n[tool: describe_dri_minerals...]\n");
    events.push(Ok(sse_event("tool_done", serde_json::json!({"name": "describe_dri_minerals"}))));

    // Mid-tool tokens
    for i in 1..=50 {
        let t = format!("after_tool1_{} ", i);
        accumulated.push_str(&t);
        events.push(Ok(sse_event("token", serde_json::json!({"delta": t}))));
    }

    // Tool 2
    events.push(Ok(sse_event("tool_start", serde_json::json!({"name": "query_dri_vitamins"}))));
    accumulated.push_str("\n[tool: query_dri_vitamins...]\n");
    events.push(Ok(sse_event("tool_done", serde_json::json!({"name": "query_dri_vitamins"}))));

    for i in 1..=50 {
        let t = format!("after_tool2_{} ", i);
        accumulated.push_str(&t);
        events.push(Ok(sse_event("token", serde_json::json!({"delta": t}))));
    }

    // Tool 3
    events.push(Ok(sse_event("tool_start", serde_json::json!({"name": "query_dri_minerals"}))));
    accumulated.push_str("\n[tool: query_dri_minerals...]\n");
    events.push(Ok(sse_event("tool_done", serde_json::json!({"name": "query_dri_minerals"}))));

    for i in 1..=50 {
        let t = format!("after_tool3_{} ", i);
        accumulated.push_str(&t);
        events.push(Ok(sse_event("token", serde_json::json!({"delta": t}))));
    }

    // Tool 4
    events.push(Ok(sse_event("tool_start", serde_json::json!({"name": "query_usda_foods"}))));
    accumulated.push_str("\n[tool: query_usda_foods...]\n");
    events.push(Ok(sse_event("tool_done", serde_json::json!({"name": "query_usda_foods"}))));

    for i in 1..=50 {
        let t = format!("after_tool4_{} ", i);
        accumulated.push_str(&t);
        events.push(Ok(sse_event("token", serde_json::json!({"delta": t}))));
    }

    // Tool 5
    events.push(Ok(sse_event("tool_start", serde_json::json!({"name": "query_who_hb"}))));
    accumulated.push_str("\n[tool: query_who_hb...]\n");
    events.push(Ok(sse_event("tool_done", serde_json::json!({"name": "query_who_hb"}))));

    // Final tokens
    for i in 1..=30 {
        let t = format!("final_{} ", i);
        accumulated.push_str(&t);
        events.push(Ok(sse_event("token", serde_json::json!({"delta": t}))));
    }

    // Done with final_text
    events.push(Ok(sse_event("done", serde_json::json!({
        "final_text": accumulated,
        "usage": {"input_tokens": 500, "output_tokens": 220}
    }))));

    events
}

/// 60 tokens then an error event — verify partial text is preserved.
fn scenario_error_mid_stream() -> Vec<Result<Event, Infallible>> {
    let mut events: Vec<Result<Event, Infallible>> = Vec::new();

    for i in 1..=60 {
        events.push(Ok(sse_event("token", serde_json::json!({"delta": format!("token_before_error_{} ", i)}))));
    }

    events.push(Ok(sse_event("error", serde_json::json!({
        "message": "API error 500: upstream LLM returned internal error after 60 tokens"
    }))));

    events
}

/// Tool returns error content but model continues — diagnostic appears in text.
fn scenario_tool_diagnostic() -> Vec<Result<Event, Infallible>> {
    let mut events: Vec<Result<Event, Infallible>> = Vec::new();
    let mut accumulated = String::new();

    // Intro tokens
    let intro = "Let me check the DRI data for you. ";
    accumulated.push_str(intro);
    events.push(Ok(sse_event("token", serde_json::json!({"delta": intro}))));

    // Tool call
    events.push(Ok(sse_event("tool_start", serde_json::json!({"name": "query_dri_minerals"}))));
    accumulated.push_str("\n[tool: query_dri_minerals...]\n");
    events.push(Ok(sse_event("tool_done", serde_json::json!({"name": "query_dri_minerals"}))));

    // Model responds to tool result with a diagnostic
    let diag = "The query returned partial results — some data for the requested age group is not available. ";
    accumulated.push_str(diag);
    events.push(Ok(sse_event("token", serde_json::json!({"delta": diag}))));

    let cont = "However, based on what we have: your daily calcium needs are approximately 1000 mg. ";
    accumulated.push_str(cont);
    events.push(Ok(sse_event("token", serde_json::json!({"delta": cont}))));

    let note = "Note: the reference range for iron may vary by laboratory. ";
    accumulated.push_str(note);
    events.push(Ok(sse_event("token", serde_json::json!({"delta": note}))));

    events.push(Ok(sse_event("done", serde_json::json!({
        "final_text": accumulated,
        "usage": {"input_tokens": 120, "output_tokens": 45}
    }))));

    events
}

/// Mixed Russian + English + emoji to verify no byte-truncation issues.
fn scenario_unicode() -> Vec<Result<Event, Infallible>> {
    let mut events: Vec<Result<Event, Infallible>> = Vec::new();
    let mut accumulated = String::new();

    let chunks = vec![
        "Привет! ",
        "Ваша суточная норма кальция ",
        "составляет 1000 мг/день. ",
        "\u{1f956} ",
        "Рекомендуемые продукты: ",
        "молоко, сыр, ",
        "брокколи \u{1f966}. ",
        "Iron needs: 8-18 mg/day. ",
        "\u{1f4ca} Vitamin D: 600-800 IU. ",
        "Всё готово! \u{2705}",
    ];

    for chunk in &chunks {
        accumulated.push_str(chunk);
        events.push(Ok(sse_event("token", serde_json::json!({"delta": chunk}))));
    }

    events.push(Ok(sse_event("done", serde_json::json!({
        "final_text": accumulated,
        "usage": {"input_tokens": 50, "output_tokens": 30}
    }))));

    events
}

/// 15 tool calls in a single response (stress-test tool rendering).
fn scenario_many_tools() -> Vec<Result<Event, Infallible>> {
    let mut events: Vec<Result<Event, Infallible>> = Vec::new();
    let mut accumulated = String::new();

    let tools = [
        "describe_dri_minerals", "describe_dri_vitamins", "describe_dri_per_kg",
        "describe_usda_foods", "describe_who_hb", "describe_who_anaemia",
        "describe_who_bmi", "describe_who_diabetes", "describe_lab_ranges",
        "query_dri_minerals", "query_dri_vitamins", "query_dri_per_kg",
        "query_usda_foods", "query_who_hb", "query_who_anaemia",
    ];

    for (i, tool) in tools.iter().enumerate() {
        let t = format!("pre_tool_{}_token ", i + 1);
        accumulated.push_str(&t);
        events.push(Ok(sse_event("token", serde_json::json!({"delta": t}))));

        let t2 = format!("about_to_call_{} ", tool);
        accumulated.push_str(&t2);
        events.push(Ok(sse_event("token", serde_json::json!({"delta": t2}))));

        events.push(Ok(sse_event("tool_start", serde_json::json!({"name": tool}))));
        accumulated.push_str(&format!("\n[tool: {}...]\n", tool));
        events.push(Ok(sse_event("tool_done", serde_json::json!({"name": tool}))));
    }

    let final_t = "All tools completed. Here is the comprehensive analysis. ";
    accumulated.push_str(final_t);
    events.push(Ok(sse_event("token", serde_json::json!({"delta": final_t}))));

    events.push(Ok(sse_event("done", serde_json::json!({
        "final_text": accumulated,
        "usage": {"input_tokens": 800, "output_tokens": 400}
    }))));

    events
}

/// Default: simple response matching the user's query.
fn scenario_simple(text: &str) -> Vec<Result<Event, Infallible>> {
    let mut events: Vec<Result<Event, Infallible>> = Vec::new();
    let response = format!("Mock response to: {}", text);

    // Split into word-sized chunks to simulate real token streaming
    for word in response.split_whitespace() {
        let chunk = format!("{} ", word);
        events.push(Ok(sse_event("token", serde_json::json!({"delta": chunk}))));
    }

    events.push(Ok(sse_event("done", serde_json::json!({
        "final_text": response,
        "usage": {"input_tokens": 10, "output_tokens": 5}
    }))));

    events
}

// ---- Handlers ----

async fn set_key_handler() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(json_ok()))
}

async fn new_chat_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut session = state.session.lock().unwrap();
    *session = MockSession {
        messages: vec![],
    };
    Ok(Json(serde_json::json!({
        "system_prompt": "You are a nutrition assistant (MOCK).",
        "message_count": 0,
        "messages": [],
        "usage": {"input_tokens": 0, "output_tokens": 0}
    })))
}

async fn send_message_handler(
    State(state): State<AppState>,
    Json(body): Json<SendMessageRequest>,
) -> Result<Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)> {
    let text = body.text.trim().to_string();
    if text.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message text is empty".into()));
    }

    let events: Vec<Result<Event, Infallible>> = build_scenario(&text);

    // Add user message to mock session
    {
        let mut session = state.session.lock().unwrap();
        session.messages.push(MockMessage {
            role: "user".into(),
            content: vec![MockContentBlock::Text { text: text.clone() }],
        });
    }

    let stream = stream::iter(events);
    Ok(Sse::new(stream))
}

async fn messages_handler() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(serde_json::json!({"messages": []})))
}

async fn save_session_handler() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(json_ok()))
}

async fn load_session_handler() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(serde_json::json!({
        "messages": [],
        "system_prompt": "You are a nutrition assistant (MOCK).",
        "message_count": 0
    })))
}

async fn clear_session_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut session = state.session.lock().unwrap();
    session.messages.clear();
    Ok(Json(json_ok()))
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8765);

    let web_dir = std::env::var("WEB_DIR").unwrap_or_else(|_| "web".into());

    let state = AppState::default();

    let api = Router::new()
        .route("/api/set_key", post(set_key_handler))
        .route("/api/new_chat", post(new_chat_handler))
        .route("/api/send_message", post(send_message_handler))
        .route("/api/messages", get(messages_handler))
        .route("/api/save_session", post(save_session_handler))
        .route("/api/load_session", post(load_session_handler))
        .route("/api/clear_session", post(clear_session_handler))
        .with_state(state);

    let app = Router::new()
        .merge(api)
        .fallback_service(ServeDir::new(&web_dir))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    println!("Mock LLM server listening on http://localhost:{}", port);
    println!("Serving web files from: {}", web_dir);

    axum::serve(listener, app).await.unwrap();
}
