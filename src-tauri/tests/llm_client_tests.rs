use dietology_lib::llm::client::LlmClient;
use dietology_lib::llm::types::*;
use dietology_lib::tools::registry::ToolRegistry;
use serde_json::json;
use std::sync::Arc;

fn make_client() -> Result<LlmClient, LlmError> {
    std::env::set_var("DEEPSEEK_API_KEY", "test-key");
    let registry = Arc::new(ToolRegistry::new());
    LlmClient::new(registry)
}

// ---- extract_tool_uses Tests ----

#[test]
fn test_extract_tool_uses_from_tool_use_response() {
    let client = make_client().unwrap();
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
    let tool_uses = client.extract_tool_uses(&response);
    assert_eq!(tool_uses.len(), 1);
    match tool_uses[0] {
        ContentBlock::ToolUse { name, .. } => assert_eq!(name, "describe_dri_minerals"),
        _ => panic!("expected ToolUse"),
    }
}

#[test]
fn test_extract_tool_uses_from_text_response() {
    let client = make_client().unwrap();
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
    let tool_uses = client.extract_tool_uses(&response);
    assert!(tool_uses.is_empty());
}

#[test]
fn test_extract_tool_uses_from_mixed_response() {
    let client = make_client().unwrap();
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
    let tool_uses = client.extract_tool_uses(&response);
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

        let tool_uses = client.extract_tool_uses(&response);
        let mut tool_results = Vec::new();
        for tu in &tool_uses {
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
