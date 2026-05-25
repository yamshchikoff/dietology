mod common;
use common::{resolve_api_key, setup_client};

use dietology_lib::llm::session::ChatSession;
use dietology_lib::llm::types::*;
use std::path::PathBuf;

// ---- Tests ----

async fn test_session_chat_roundtrip() {
    let (client, system_prompt) = setup_client().expect("failed to create client");
    let mut session = ChatSession::new(system_prompt);

    assert_eq!(session.message_count(), 0);
    session.add_user_message("Сколько кальция рекомендуется мужчине 19-30 лет?".to_string());

    let response = client
        .chat(&mut session.messages, &session.system_prompt)
        .await
        .expect("chat() failed");

    // Сессия пополнилась сообщениями от модели
    assert!(
        session.message_count() > 1,
        "expected >1 messages after chat, got {}",
        session.message_count()
    );

    // Аккумулируем usage
    session.add_usage(response.usage);
    assert!(session.total_usage.input_tokens > 0);
    assert!(session.total_usage.output_tokens > 0);

    // Проверяем цепочку через сессию
    assert_eq!(session.messages[0].role, "user");
    let last = session.messages.last().unwrap();
    assert_eq!(last.role, "assistant");

    let has_tool_use = session.messages.iter().any(|m| {
        m.content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }))
    });
    assert!(has_tool_use);

    eprintln!(
        "SUCCESS: session has {} messages, usage in={} out={}",
        session.message_count(),
        session.total_usage.input_tokens,
        session.total_usage.output_tokens
    );
}

async fn test_session_save_load_after_chat() {
    let (client, system_prompt) = setup_client().expect("failed to create client");
    let mut session = ChatSession::new(system_prompt.clone());
    session.add_user_message("Какие пороги гемоглобина для диагностики анемии у беременных?".to_string());

    client
        .chat(&mut session.messages, &session.system_prompt)
        .await
        .expect("chat() failed");

    let msg_count_before = session.message_count();
    assert!(msg_count_before > 1);

    // Сохраняем
    let tmp_path = PathBuf::from("/tmp/test_dietology_session_integration.jsonl");
    session.save_to_jsonl(&tmp_path).expect("save_to_jsonl failed");

    // Загружаем
    let loaded = ChatSession::load_from_jsonl(&tmp_path).expect("load_from_jsonl failed");

    assert_eq!(loaded.system_prompt, system_prompt);
    assert_eq!(loaded.message_count(), msg_count_before);
    assert_eq!(loaded.messages[0].role, "user");

    // Usage не сохраняется — нулевой после загрузки
    assert_eq!(loaded.total_usage.input_tokens, 0);
    assert_eq!(loaded.total_usage.output_tokens, 0);

    // Проверяем tool_use/tool_result в сохранённых сообщениях
    let has_tool_use = loaded.messages.iter().any(|m| {
        m.content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }))
    });
    assert!(has_tool_use, "saved messages should contain tool_use");

    let has_tool_result = loaded.messages.iter().any(|m| {
        m.content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolResult { .. }))
    });
    assert!(
        has_tool_result,
        "saved messages should contain tool_result"
    );

    std::fs::remove_file(&tmp_path).ok();
    eprintln!("SUCCESS: saved and loaded {msg_count_before} messages");
}

// ---- Entrypoint ----

fn main() {
    let Some(key) = resolve_api_key() else {
        eprintln!("Usage: cargo test --test llm_session_integration -- --api-key <KEY>");
        eprintln!("   or: DEEPSEEK_API_KEY=sk-... cargo test --test llm_session_integration");
        std::process::exit(1);
    };
    std::env::set_var("DEEPSEEK_API_KEY", key);

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    let results = [
        run_test("chat_roundtrip", || {
            rt.block_on(test_session_chat_roundtrip())
        }),
        run_test("save_load", || {
            rt.block_on(test_session_save_load_after_chat())
        }),
    ];
    let passed = results.iter().filter(|&&r| r).count();
    let failed = results.len() - passed;
    eprintln!("--- {passed} passed, {failed} failed ---");
    if failed > 0 {
        std::process::exit(1);
    }
}
