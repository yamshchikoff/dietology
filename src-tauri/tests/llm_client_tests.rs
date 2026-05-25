use dietology_lib::llm::client::{extract_tool_uses, LlmClient};
use dietology_lib::llm::types::*;
use dietology_lib::tools::registry::ToolRegistry;
use serde_json::json;
use std::sync::Arc;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn make_client() -> Result<LlmClient, LlmError> {
    let _lock = ENV_LOCK.lock().unwrap();
    std::env::set_var("DEEPSEEK_API_KEY", "test-key");
    let registry = Arc::new(ToolRegistry::new());
    LlmClient::new(registry)
}

// ---- extract_tool_uses Tests ----

#[test]
fn test_extract_tool_uses_from_tool_use_response() {
    let _client = make_client().unwrap();
    let response = ApiResponse {
        id: "msg_001".into(),
        msg_type: "message".into(),
        role: "assistant".into(),
        content: vec![ContentBlock::ToolUse {
            id: "toolu_001".into(),
            name: "describe_dri_minerals".into(),
            input: json!({}),
        }],
        stop_reason: "tool_use".into(),
        usage: Usage {
            input_tokens: 100,
            output_tokens: 50,
        },
    };
    let tool_uses = extract_tool_uses(&response.content);
    assert_eq!(tool_uses.len(), 1);
    match tool_uses[0] {
        ContentBlock::ToolUse { name, .. } => assert_eq!(name, "describe_dri_minerals"),
        _ => panic!("expected ToolUse"),
    }
}

#[test]
fn test_extract_tool_uses_from_text_response() {
    let _client = make_client().unwrap();
    let response = ApiResponse {
        id: "msg_002".into(),
        msg_type: "message".into(),
        role: "assistant".into(),
        content: vec![ContentBlock::Text {
            text: "Answer.".into(),
        }],
        stop_reason: "end_turn".into(),
        usage: Usage {
            input_tokens: 100,
            output_tokens: 50,
        },
    };
    let tool_uses = extract_tool_uses(&response.content);
    assert!(tool_uses.is_empty());
}

#[test]
fn test_extract_tool_uses_from_mixed_response() {
    let _client = make_client().unwrap();
    let response = ApiResponse {
        id: "msg_003".into(),
        msg_type: "message".into(),
        role: "assistant".into(),
        content: vec![
            ContentBlock::Text {
                text: "Let me check.".into(),
            },
            ContentBlock::ToolUse {
                id: "toolu_002".into(),
                name: "query_dri_minerals".into(),
                input: json!({"nutrient": "Zinc"}),
            },
        ],
        stop_reason: "tool_use".into(),
        usage: Usage {
            input_tokens: 150,
            output_tokens: 60,
        },
    };
    let tool_uses = extract_tool_uses(&response.content);
    assert_eq!(tool_uses.len(), 1);
}

// ---- dispatch_tool Tests ----

#[test]
fn test_dispatch_tool_with_registered_handler() {
    let mut registry = ToolRegistry::new();
    registry.register(
        "test_tool",
        "A test tool",
        json!({"type": "object", "properties": {}, "required": []}),
        Box::new(|args| {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            Ok(format!(r#"{{"status":"ok","name":"{}"}}"#, name))
        }),
    );
    let client = LlmClient {
        api_base_url: "https://api.deepseek.com".into(),
        api_key: "test-key".into(),
        model: "deepseek-chat".into(),
        http: reqwest::Client::new(),
        http_stream: reqwest::Client::new(),
        registry: Arc::new(registry),
        max_tokens: 4096,
        max_tool_rounds: 10,
    };

    let tool_use = ContentBlock::ToolUse {
        id: "toolu_001".into(),
        name: "test_tool".into(),
        input: json!({"name": "Alice"}),
    };
    let result = client.dispatch_tool(&tool_use).unwrap();
    assert!(result.contains("Alice"));
    assert!(result.contains("ok"));
}

#[test]
fn test_dispatch_tool_unknown_tool_returns_error() {
    let client = make_client().unwrap();
    let tool_use = ContentBlock::ToolUse {
        id: "toolu_001".into(),
        name: "nonexistent_tool".into(),
        input: json!({}),
    };
    let result = client.dispatch_tool(&tool_use);
    assert!(result.is_err());
    match result {
        Err(LlmError::ToolDispatch(msg)) => assert!(msg.contains("nonexistent_tool")),
        _ => panic!("expected ToolDispatch error"),
    }
}

#[test]
fn test_dispatch_tool_with_text_block_returns_error() {
    let client = make_client().unwrap();
    let text_block = ContentBlock::Text {
        text: "not a tool".into(),
    };
    let result = client.dispatch_tool(&text_block);
    match result {
        Err(LlmError::ToolDispatch(_)) => {} // expected
        other => panic!("expected ToolDispatch error, got {other:?}"),
    }
}

// ---- Client Construction Tests ----

#[test]
fn test_client_new_missing_api_key() {
    let _lock = ENV_LOCK.lock().unwrap();
    std::env::remove_var("DEEPSEEK_API_KEY");
    let registry = Arc::new(ToolRegistry::new());
    let result = LlmClient::new(registry);
    match result {
        Err(LlmError::MissingApiKey) => {}
        _ => panic!("expected MissingApiKey"),
    }
    std::env::set_var("DEEPSEEK_API_KEY", "test-key");
}

#[test]
fn test_client_new_with_custom_base_url() {
    let _lock = ENV_LOCK.lock().unwrap();
    std::env::set_var("DEEPSEEK_API_KEY", "test-key");
    std::env::set_var("DEEPSEEK_API_BASE", "https://custom.api.com");
    std::env::set_var("DEEPSEEK_MODEL", "custom-model");

    let registry = Arc::new(ToolRegistry::new());
    let client = LlmClient::new(registry).unwrap();
    assert_eq!(client.api_base_url, "https://custom.api.com");
    assert_eq!(client.model, "custom-model");

    std::env::remove_var("DEEPSEEK_API_BASE");
    std::env::remove_var("DEEPSEEK_MODEL");
}

// ---- Tool Loop Max Rounds Test ----

/// Ручная симуляция 3 раундов tool_use с max_tool_rounds=2.
/// Проверяет, что цикл накапливает сообщения и диспатчит инструменты
/// — паттерн, который chat() использует внутри.
#[test]
fn test_tool_loop_max_rounds_manual_simulation() {
    let mut registry = ToolRegistry::new();
    registry.register(
        "echo",
        "Echo tool",
        json!({"type": "object", "properties": {}, "required": []}),
        Box::new(|_args| Ok(r#"{"status":"ok"}"#.into())),
    );

    let client = LlmClient {
        api_base_url: "https://api.deepseek.com".into(),
        api_key: "test-key".into(),
        model: "deepseek-chat".into(),
        http: reqwest::Client::new(),
        http_stream: reqwest::Client::new(),
        registry: Arc::new(registry),
        max_tokens: 4096,
        max_tool_rounds: 2,
    };

    let mut messages: Vec<Message> = vec![];
    let mut rounds = 0;

    // Симулируем 3 раунда (превышает max_tool_rounds=2)
    loop {
        rounds += 1;
        if rounds > 3 {
            break;
        }

        let response = ApiResponse {
            id: format!("msg_{rounds}"),
            msg_type: "message".into(),
            role: "assistant".into(),
            content: vec![ContentBlock::ToolUse {
                id: format!("toolu_{rounds}"),
                name: "echo".into(),
                input: json!({}),
            }],
            stop_reason: "tool_use".into(),
            usage: Usage {
                input_tokens: 10,
                output_tokens: 5,
            },
        };

        let tool_uses = extract_tool_uses(&response.content);
        let mut tool_results = Vec::new();
        for tu in tool_uses {
            if let ContentBlock::ToolUse { id, .. } = tu {
                let result = client.dispatch_tool(tu).unwrap();
                tool_results.push(ContentBlock::ToolResult {
                    tool_use_id: id.clone(),
                    content: result,
                });
            }
        }

        messages.push(Message {
            role: "assistant".into(),
            content: response.content,
        });
        messages.push(Message {
            role: "user".into(),
            content: tool_results,
        });
    }

    assert_eq!(messages.len(), 6); // 3 ассистент + 3 user с tool_result
    assert!(messages.iter().any(|m| m.role == "user"
        && m.content.iter().any(|b| matches!(b, ContentBlock::ToolResult { .. }))));
}

// ---- call_api() Tests (wiremock) ----

#[tokio::test]
async fn test_call_api_end_turn() {
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_001",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Ответ модели."}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        })))
        .mount(&server)
        .await;

    let client = LlmClient {
        api_base_url: server.uri(),
        api_key: "test-key".into(),
        model: "deepseek-chat".into(),
        http: reqwest::Client::new(),
        http_stream: reqwest::Client::new(),
        registry: Arc::new(ToolRegistry::new()),
        max_tokens: 4096,
        max_tool_rounds: 10,
    };

    let response = client.call_api(&[], "system").await.unwrap();
    assert_eq!(response.stop_reason, "end_turn");
    assert_eq!(response.usage.input_tokens, 10);
}

#[tokio::test]
async fn test_call_api_http_error() {
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&server)
        .await;

    let client = LlmClient {
        api_base_url: server.uri(),
        api_key: "test-key".into(),
        model: "deepseek-chat".into(),
        http: reqwest::Client::new(),
        http_stream: reqwest::Client::new(),
        registry: Arc::new(ToolRegistry::new()),
        max_tokens: 4096,
        max_tool_rounds: 10,
    };

    let result = client.call_api(&[], "system").await;
    match result {
        Err(LlmError::Api { status, .. }) => assert_eq!(status, 500),
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_call_api_invalid_json() {
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let client = LlmClient {
        api_base_url: server.uri(),
        api_key: "test-key".into(),
        model: "deepseek-chat".into(),
        http: reqwest::Client::new(),
        http_stream: reqwest::Client::new(),
        registry: Arc::new(ToolRegistry::new()),
        max_tokens: 4096,
        max_tool_rounds: 10,
    };

    let result = client.call_api(&[], "system").await;
    match result {
        Err(LlmError::Parse(_)) => {}
        other => panic!("expected Parse error, got {other:?}"),
    }
}

// ---- chat() Tests (wiremock) ----

#[tokio::test]
async fn test_chat_end_turn() {
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_001",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Финальный ответ."}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 20, "output_tokens": 10}
        })))
        .mount(&server)
        .await;

    let client = LlmClient {
        api_base_url: server.uri(),
        api_key: "test-key".into(),
        model: "deepseek-chat".into(),
        http: reqwest::Client::new(),
        http_stream: reqwest::Client::new(),
        registry: Arc::new(ToolRegistry::new()),
        max_tokens: 4096,
        max_tool_rounds: 10,
    };

    let mut messages = vec![Message {
        role: "user".into(),
        content: vec![ContentBlock::Text { text: "вопрос".into() }],
    }];

    let response = client.chat(&mut messages, "system").await.unwrap();
    assert!(!response.final_text.is_empty());
    assert_eq!(response.usage.input_tokens, 20);
    assert_eq!(response.usage.output_tokens, 10);
    // user + assistant = 2 сообщения
    assert_eq!(messages.len(), 2);
    assert_eq!(messages.last().unwrap().role, "assistant");
}

#[tokio::test]
async fn test_chat_tool_use_then_end_turn() {
    use std::sync::Mutex;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    let mut registry = ToolRegistry::new();
    registry.register(
        "echo",
        "Echo tool",
        serde_json::json!({"type": "object", "properties": {}, "required": []}),
        Box::new(|_args| Ok(r#"{"status":"ok"}"#.into())),
    );

    let server = MockServer::start().await;
    let call_count = Arc::new(Mutex::new(0u32));
    let count = call_count.clone();

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(move |_req: &wiremock::Request| {
            let mut n = count.lock().unwrap();
            *n += 1;
            if *n == 1 {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "msg_001",
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "tool_use", "id": "toolu_001", "name": "echo", "input": {}}],
                    "stop_reason": "tool_use",
                    "usage": {"input_tokens": 30, "output_tokens": 15}
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "msg_002",
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Готово."}],
                    "stop_reason": "end_turn",
                    "usage": {"input_tokens": 40, "output_tokens": 20}
                }))
            }
        })
        .mount(&server)
        .await;

    let client = LlmClient {
        api_base_url: server.uri(),
        api_key: "test-key".into(),
        model: "deepseek-chat".into(),
        http: reqwest::Client::new(),
        http_stream: reqwest::Client::new(),
        registry: Arc::new(registry),
        max_tokens: 4096,
        max_tool_rounds: 10,
    };

    let mut messages = vec![Message {
        role: "user".into(),
        content: vec![ContentBlock::Text { text: "вопрос".into() }],
    }];

    let response = client.chat(&mut messages, "system").await.unwrap();
    assert!(!response.final_text.is_empty());
    assert_eq!(response.usage.input_tokens, 70);
    assert_eq!(response.usage.output_tokens, 35);
    assert_eq!(messages.len(), 4);
    assert!(messages.iter().any(|m| m.content.iter().any(|b| matches!(b, ContentBlock::ToolUse { .. }))));
    assert!(messages.iter().any(|m| m.content.iter().any(|b| matches!(b, ContentBlock::ToolResult { .. }))));
}

#[tokio::test]
async fn test_chat_max_tokens_is_success() {
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_001",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Ответ до лимита."}],
            "stop_reason": "max_tokens",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        })))
        .mount(&server)
        .await;

    let client = LlmClient {
        api_base_url: server.uri(),
        api_key: "test-key".into(),
        model: "deepseek-chat".into(),
        http: reqwest::Client::new(),
        http_stream: reqwest::Client::new(),
        registry: Arc::new(ToolRegistry::new()),
        max_tokens: 4096,
        max_tool_rounds: 10,
    };

    let mut messages = vec![Message {
        role: "user".into(),
        content: vec![ContentBlock::Text { text: "вопрос".into() }],
    }];

    let response = client.chat(&mut messages, "system").await.unwrap();
    assert!(!response.final_text.is_empty());
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(messages.len(), 2);
}

#[tokio::test]
async fn test_chat_end_turn_empty_text_returns_error() {
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_001",
            "type": "message",
            "role": "assistant",
            "content": [],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        })))
        .mount(&server)
        .await;

    let client = LlmClient {
        api_base_url: server.uri(),
        api_key: "test-key".into(),
        model: "deepseek-chat".into(),
        http: reqwest::Client::new(),
        http_stream: reqwest::Client::new(),
        registry: Arc::new(ToolRegistry::new()),
        max_tokens: 4096,
        max_tool_rounds: 10,
    };

    let mut messages = vec![Message {
        role: "user".into(),
        content: vec![ContentBlock::Text { text: "вопрос".into() }],
    }];

    let result = client.chat(&mut messages, "system").await;
    match result {
        Err(LlmError::Parse(msg)) => assert!(msg.contains("no text")),
        other => panic!("expected Parse error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_chat_unknown_stop_reason() {
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_001",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "..."}],
            "stop_reason": "refusal",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        })))
        .mount(&server)
        .await;

    let client = LlmClient {
        api_base_url: server.uri(),
        api_key: "test-key".into(),
        model: "deepseek-chat".into(),
        http: reqwest::Client::new(),
        http_stream: reqwest::Client::new(),
        registry: Arc::new(ToolRegistry::new()),
        max_tokens: 4096,
        max_tool_rounds: 10,
    };

    let mut messages = vec![Message {
        role: "user".into(),
        content: vec![ContentBlock::Text { text: "вопрос".into() }],
    }];

    let result = client.chat(&mut messages, "system").await;
    match result {
        Err(LlmError::Parse(msg)) => assert!(msg.contains("refusal")),
        other => panic!("expected Parse error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_chat_max_tool_rounds() {
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    let mut registry = ToolRegistry::new();
    registry.register(
        "echo",
        "Echo tool",
        serde_json::json!({"type": "object", "properties": {}, "required": []}),
        Box::new(|_args| Ok(r#"{"status":"ok"}"#.into())),
    );

    let server = MockServer::start().await;
    // Всегда возвращаем tool_use — бесконечный цикл до MaxToolRounds
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_001",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "tool_use", "id": "toolu_001", "name": "echo", "input": {}}],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        })))
        .mount(&server)
        .await;

    let client = LlmClient {
        api_base_url: server.uri(),
        api_key: "test-key".into(),
        model: "deepseek-chat".into(),
        http: reqwest::Client::new(),
        http_stream: reqwest::Client::new(),
        registry: Arc::new(registry),
        max_tokens: 4096,
        max_tool_rounds: 2,
    };

    let mut messages = vec![Message {
        role: "user".into(),
        content: vec![ContentBlock::Text { text: "вопрос".into() }],
    }];

    let result = client.chat(&mut messages, "system").await;
    match result {
        Err(LlmError::MaxToolRounds { rounds: 2, messages: err_msgs }) => {
            assert_eq!(err_msgs.len(), 5, "error should carry accumulated messages");
        }
        other => panic!("expected MaxToolRounds(2), got {other:?}"),
    }
    // 2 раунда: user + 2*(assistant + user_tool_result) = 5 сообщений
    assert_eq!(messages.len(), 5);
}
