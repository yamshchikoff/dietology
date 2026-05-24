use dietology_lib::llm::session::ChatSession;
use dietology_lib::llm::types::{ContentBlock, Message, Usage};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn test_new_session() {
    let session = ChatSession::new("You are a nutrition assistant.".into());
    assert!(session.messages.is_empty());
    assert_eq!(session.system_prompt, "You are a nutrition assistant.");
    assert_eq!(session.total_usage.input_tokens, 0);
    assert_eq!(session.total_usage.output_tokens, 0);
}

#[test]
fn test_add_user_message() {
    let mut session = ChatSession::new("system".into());
    session.add_user_message("Hello".into());
    assert_eq!(session.message_count(), 1);
    assert_eq!(session.messages[0].role, "user");
}

#[test]
fn test_add_usage() {
    let mut session = ChatSession::new("system".into());
    session.add_usage(&Usage {
        input_tokens: 100,
        output_tokens: 50,
    });
    session.add_usage(&Usage {
        input_tokens: 200,
        output_tokens: 80,
    });
    assert_eq!(session.total_usage.input_tokens, 300);
    assert_eq!(session.total_usage.output_tokens, 130);
}

#[test]
fn test_clear() {
    let mut session = ChatSession::new("system".into());
    session.add_user_message("Hello".into());
    session.add_usage(&Usage {
        input_tokens: 100,
        output_tokens: 50,
    });
    session.clear();
    assert!(session.messages.is_empty());
    assert_eq!(session.total_usage.input_tokens, 0);
    assert_eq!(session.system_prompt, "system"); // промпт сохраняется
}

#[test]
fn test_save_and_load_jsonl() {
    let mut session = ChatSession::new("You are a nutrition assistant.".into());
    session.add_user_message("Сколько кальция?".into());

    // Добавляем искусственный ответ ассистента
    session.messages.push(Message {
        role: "assistant".into(),
        content: vec![ContentBlock::Text {
            text: "Рекомендация: 1000 mg/день.".into(),
        }],
    });

    let tmp_path = PathBuf::from("/tmp/test_dietology_session.jsonl");

    session.save_to_jsonl(&tmp_path).unwrap();

    let loaded = ChatSession::load_from_jsonl(&tmp_path).unwrap();
    assert_eq!(loaded.system_prompt, "You are a nutrition assistant.");
    assert_eq!(loaded.message_count(), 2);
    assert_eq!(loaded.messages[0].role, "user");
    assert_eq!(loaded.messages[1].role, "assistant");

    // Usage не сохраняется — после загрузки нулевой
    assert_eq!(loaded.total_usage.input_tokens, 0);

    // Убираем за собой
    std::fs::remove_file(&tmp_path).ok();
}

#[test]
fn test_save_and_load_with_tool_use() {
    let mut session = ChatSession::new("system".into());
    session.add_user_message("query".into());

    session.messages.push(Message {
        role: "assistant".into(),
        content: vec![
            ContentBlock::Text {
                text: "Let me check.".into(),
            },
            ContentBlock::ToolUse {
                id: "toolu_001".into(),
                name: "describe_dri_minerals".into(),
                input: json!({}),
            },
        ],
    });

    session.messages.push(Message {
        role: "user".into(),
        content: vec![ContentBlock::ToolResult {
            tool_use_id: "toolu_001".into(),
            content: r#"{"status":"ok","nutrients":["Calcium"]}"#.into(),
        }],
    });

    let tmp_path = PathBuf::from("/tmp/test_dietology_session_tools.jsonl");

    session.save_to_jsonl(&tmp_path).unwrap();

    let loaded = ChatSession::load_from_jsonl(&tmp_path).unwrap();
    assert_eq!(loaded.message_count(), 3);

    // Проверяем, что tool_use сохранился корректно
    let assistant_msg = &loaded.messages[1];
    assert_eq!(assistant_msg.role, "assistant");
    let has_tool_use = assistant_msg
        .content
        .iter()
        .any(|b| matches!(b, ContentBlock::ToolUse { .. }));
    assert!(has_tool_use);

    // Проверяем tool_result
    let tool_result_msg = &loaded.messages[2];
    assert_eq!(tool_result_msg.role, "user");
    let has_tool_result = tool_result_msg
        .content
        .iter()
        .any(|b| matches!(b, ContentBlock::ToolResult { .. }));
    assert!(has_tool_result);

    std::fs::remove_file(&tmp_path).ok();
}

#[test]
fn test_load_nonexistent_file() {
    let path = PathBuf::from("/tmp/nonexistent_dietology_session.jsonl");
    let result = ChatSession::load_from_jsonl(&path);
    assert!(result.is_err());
}

#[test]
fn test_message_count() {
    let mut session = ChatSession::new("system".into());
    assert_eq!(session.message_count(), 0);
    session.add_user_message("msg1".into());
    session.add_user_message("msg2".into());
    assert_eq!(session.message_count(), 2);
}
