use dietology_lib::data::DataLoader;
use dietology_lib::llm::client::LlmClient;
use dietology_lib::llm::types::*;
use dietology_lib::tools::registry::ToolRegistry;
use std::sync::Arc;

// ---- Helpers ----

fn setup_client() -> Result<(LlmClient, String), LlmError> {
    let loader = DataLoader::for_development();
    let mut registry = ToolRegistry::new();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);
    dietology_lib::tools::query::register_query_tools(&mut registry, &loader);
    let client = LlmClient::new(Arc::new(registry))?;
    let system_prompt = "\
Ты — ассистент по питанию. Отвечай на русском языке.
Для поиска данных используй инструменты: сначала describe для навигации, потом query для конкретных значений."
        .into();
    Ok((client, system_prompt))
}

fn user_message(text: &str) -> Message {
    Message {
        role: "user".into(),
        content: vec![ContentBlock::Text {
            text: text.into(),
        }],
    }
}

fn assert_valid_response(resp: &LlmResponse, messages: &[Message]) {
    assert!(
        !resp.final_text.is_empty(),
        "final_text should not be empty"
    );

    assert!(
        messages.len() >= 4,
        "expected at least 4 messages: user, assistant(tool_use), user(tool_result), assistant(text). Got {}",
        messages.len()
    );

    assert_eq!(messages[0].role, "user");

    let has_tool_use = messages.iter().any(|m| {
        m.content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }))
    });
    assert!(
        has_tool_use,
        "expected at least one tool_use in message history"
    );

    let has_tool_result = messages.iter().any(|m| {
        m.content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolResult { .. }))
    });
    assert!(
        has_tool_result,
        "expected at least one tool_result in message history"
    );

    let last = messages.last().unwrap();
    assert_eq!(last.role, "assistant");

    assert!(resp.usage.input_tokens > 0);
    assert!(resp.usage.output_tokens > 0);

    let preview: String = resp.final_text.chars().take(200).collect();
    eprintln!("SUCCESS: final_text = {preview}");
}

fn resolve_api_key() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--api-key") {
        if let Some(val) = args.get(pos + 1) {
            if !val.starts_with("--") {
                return Some(val.clone());
            }
        }
    }
    std::env::var("DEEPSEEK_API_KEY").ok()
}

// ---- Tests ----

async fn test_full_roundtrip_calcium() {
    let (client, system_prompt) = setup_client().expect("failed to create client");
    let mut messages = vec![user_message(
        "Сколько кальция рекомендуется мужчине 19-30 лет?",
    )];

    let response = client.chat(&mut messages, &system_prompt).await;
    match response {
        Ok(resp) => assert_valid_response(&resp, &messages),
        Err(e) => panic!("chat() failed: {e}"),
    }
}

async fn test_roundtrip_vitamin_c() {
    let (client, system_prompt) = setup_client().expect("failed to create client");
    let mut messages = vec![user_message(
        "Сколько витамина C рекомендуется женщине 19-30 лет?",
    )];

    let response = client.chat(&mut messages, &system_prompt).await;
    match response {
        Ok(resp) => assert_valid_response(&resp, &messages),
        Err(e) => panic!("chat() failed: {e}"),
    }
}

async fn test_roundtrip_who_hb() {
    let (client, system_prompt) = setup_client().expect("failed to create client");
    let mut messages = vec![user_message(
        "Какие пороги гемоглобина для диагностики анемии у беременных?",
    )];

    let response = client.chat(&mut messages, &system_prompt).await;
    match response {
        Ok(resp) => assert_valid_response(&resp, &messages),
        Err(e) => panic!("chat() failed: {e}"),
    }
}

async fn test_roundtrip_usda_milk() {
    let (client, system_prompt) = setup_client().expect("failed to create client");
    let mut messages = vec![user_message(
        "Сколько кальция содержится в коровьем молоке?",
    )];

    let response = client.chat(&mut messages, &system_prompt).await;
    match response {
        Ok(resp) => assert_valid_response(&resp, &messages),
        Err(e) => panic!("chat() failed: {e}"),
    }
}

async fn test_roundtrip_lab_ranges() {
    let (client, system_prompt) = setup_client().expect("failed to create client");
    let mut messages = vec![user_message(
        "Какие референсные значения гемоглобина в крови?",
    )];

    let response = client.chat(&mut messages, &system_prompt).await;
    match response {
        Ok(resp) => assert_valid_response(&resp, &messages),
        Err(e) => panic!("chat() failed: {e}"),
    }
}

// ---- Entrypoint ----

fn main() {
    let Some(key) = resolve_api_key() else {
        eprintln!("Usage: cargo test --test llm_chat_integration -- --api-key <KEY>");
        eprintln!("   or: DEEPSEEK_API_KEY=sk-... cargo test --test llm_chat_integration");
        std::process::exit(1);
    };
    std::env::set_var("DEEPSEEK_API_KEY", key);

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    let results = [
        run_test("calcium", || rt.block_on(test_full_roundtrip_calcium())),
        run_test("vitamin_c", || rt.block_on(test_roundtrip_vitamin_c())),
        run_test("who_hb", || rt.block_on(test_roundtrip_who_hb())),
        run_test("usda_milk", || rt.block_on(test_roundtrip_usda_milk())),
        run_test("lab_ranges", || rt.block_on(test_roundtrip_lab_ranges())),
    ];
    let passed = results.iter().filter(|&&r| r).count();
    let failed = results.len() - passed;
    eprintln!("--- {passed} passed, {failed} failed ---");
    if failed > 0 {
        std::process::exit(1);
    }
}

fn run_test<F: FnOnce()>(name: &str, f: F) -> bool {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(()) => {
            eprintln!("PASS: {name}");
            true
        }
        Err(_) => {
            eprintln!("FAIL: {name} (panic)");
            false
        }
    }
}
