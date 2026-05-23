use dietology_lib::llm::types::*;
use dietology_lib::tools::registry::ToolDefinition;
use serde_json::json;

// ---- ContentBlock Tests ----

#[test]
fn test_content_block_text_serialization() {
    let block = ContentBlock::Text {
        text: "hello".into(),
    };
    let json = serde_json::to_value(&block).unwrap();
    assert_eq!(json["type"], "text");
    assert_eq!(json["text"], "hello");
}

#[test]
fn test_content_block_text_deserialization() {
    let json = json!({"type": "text", "text": "hello"});
    let block: ContentBlock = serde_json::from_value(json).unwrap();
    match block {
        ContentBlock::Text { text } => assert_eq!(text, "hello"),
        _ => panic!("expected Text"),
    }
}

#[test]
fn test_content_block_tool_use_serialization() {
    let block = ContentBlock::ToolUse {
        id: "toolu_001".into(),
        name: "describe_dri_minerals".into(),
        input: json!({}),
    };
    let json = serde_json::to_value(&block).unwrap();
    assert_eq!(json["type"], "tool_use");
    assert_eq!(json["id"], "toolu_001");
    assert_eq!(json["name"], "describe_dri_minerals");
}

#[test]
fn test_content_block_tool_use_deserialization() {
    let json = json!({
        "type": "tool_use",
        "id": "toolu_001",
        "name": "query_dri_minerals",
        "input": {"nutrient": "Calcium"}
    });
    let block: ContentBlock = serde_json::from_value(json).unwrap();
    match block {
        ContentBlock::ToolUse { id, name, input } => {
            assert_eq!(id, "toolu_001");
            assert_eq!(name, "query_dri_minerals");
            assert_eq!(input["nutrient"], "Calcium");
        }
        _ => panic!("expected ToolUse"),
    }
}

#[test]
fn test_content_block_tool_result_roundtrip() {
    let block = ContentBlock::ToolResult {
        tool_use_id: "toolu_001".into(),
        content: r#"{"status":"ok","nutrients":["Calcium"]}"#.into(),
    };
    let json = serde_json::to_value(&block).unwrap();
    assert_eq!(json["type"], "tool_result");
    let back: ContentBlock = serde_json::from_value(json).unwrap();
    match back {
        ContentBlock::ToolResult {
            tool_use_id,
            content,
        } => {
            assert_eq!(tool_use_id, "toolu_001");
            assert!(content.contains("Calcium"));
        }
        _ => panic!("expected ToolResult"),
    }
}

#[test]
fn test_content_block_unknown_type_is_error() {
    let json = json!({"type": "unknown_variant", "text": "x"});
    let result: Result<ContentBlock, _> = serde_json::from_value(json);
    assert!(result.is_err());
}

// ---- Message Tests ----

#[test]
fn test_message_roundtrip() {
    let msg = Message {
        role: "assistant".into(),
        content: vec![
            ContentBlock::Text {
                text: "Answer:".into(),
            },
            ContentBlock::ToolUse {
                id: "toolu_001".into(),
                name: "query_dri_minerals".into(),
                input: json!({"nutrient": "Calcium"}),
            },
        ],
    };
    let json = serde_json::to_value(&msg).unwrap();
    assert_eq!(json["role"], "assistant");
    assert_eq!(json["content"].as_array().unwrap().len(), 2);
    let back: Message = serde_json::from_value(json).unwrap();
    assert_eq!(back.role, "assistant");
    assert_eq!(back.content.len(), 2);
}

// ---- ApiRequest Test ----

#[test]
fn test_api_request_serialization() {
    let req = ApiRequest {
        model: "deepseek-chat".into(),
        max_tokens: 4096,
        system: "You are a nutrition assistant.".into(),
        messages: vec![Message {
            role: "user".into(),
            content: vec![ContentBlock::Text {
                text: "Hello".into(),
            }],
        }],
        tools: vec![ToolDefinition {
            name: "describe_dri_minerals".into(),
            description: "List DRI minerals".into(),
            input_schema: json!({"type": "object", "properties": {}, "required": []}),
        }],
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["model"], "deepseek-chat");
    assert_eq!(json["max_tokens"], 4096);
    assert!(json["system"].as_str().unwrap().contains("nutrition"));
    assert_eq!(json["messages"].as_array().unwrap().len(), 1);
    assert_eq!(json["tools"].as_array().unwrap().len(), 1);
    assert_eq!(json["tools"][0]["name"], "describe_dri_minerals");
}

// ---- ApiResponse Tests ----

#[test]
fn test_api_response_deserialization_tool_use() {
    let json = json!({
        "id": "msg_001",
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "tool_use", "id": "toolu_001", "name": "describe_dri_minerals", "input": {}}
        ],
        "stop_reason": "tool_use",
        "usage": {"input_tokens": 100, "output_tokens": 50}
    });
    let resp: ApiResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.id, "msg_001");
    assert_eq!(resp.stop_reason, "tool_use");
    assert_eq!(resp.usage.input_tokens, 100);
    assert_eq!(resp.usage.output_tokens, 50);
    match &resp.content[0] {
        ContentBlock::ToolUse { id, name, .. } => {
            assert_eq!(id, "toolu_001");
            assert_eq!(name, "describe_dri_minerals");
        }
        _ => panic!("expected ToolUse"),
    }
}

#[test]
fn test_api_response_deserialization_end_turn() {
    let json = json!({
        "id": "msg_002",
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "text", "text": "Рекомендация: 1000 mg/день."}
        ],
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 200, "output_tokens": 80}
    });
    let resp: ApiResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.stop_reason, "end_turn");
    match &resp.content[0] {
        ContentBlock::Text { text } => assert!(text.contains("1000 mg")),
        _ => panic!("expected Text"),
    }
}

#[test]
fn test_api_response_deserialization_mixed_content() {
    let json = json!({
        "id": "msg_003",
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "text", "text": "Let me check."},
            {"type": "tool_use", "id": "toolu_002", "name": "query_dri_minerals", "input": {"nutrient": "Zinc"}}
        ],
        "stop_reason": "tool_use",
        "usage": {"input_tokens": 150, "output_tokens": 60}
    });
    let resp: ApiResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.content.len(), 2);
    match &resp.content[0] {
        ContentBlock::Text { text } => assert_eq!(text, "Let me check."),
        _ => panic!("expected Text"),
    }
    match &resp.content[1] {
        ContentBlock::ToolUse { name, .. } => assert_eq!(name, "query_dri_minerals"),
        _ => panic!("expected ToolUse"),
    }
}

// ---- LlmError Test ----

#[test]
fn test_llm_error_variants() {
    let _e1 = LlmError::Network("timeout".into());
    let _e2 = LlmError::Api {
        status: 500,
        body: "Internal Server Error".into(),
    };
    let _e3 = LlmError::Parse("invalid json".into());
    let _e4 = LlmError::ToolDispatch("unknown tool".into());
    let _e5 = LlmError::MaxToolRounds(10);
    let _e6 = LlmError::MissingApiKey;
}
