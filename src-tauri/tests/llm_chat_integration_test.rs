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

    eprintln!(
        "SUCCESS: final_text = {}",
        &resp.final_text[..200.min(resp.final_text.len())]
    );
}

fn api_key_present() -> bool {
    std::env::var("DEEPSEEK_API_KEY").is_ok()
}

// ---- Tests ----

/// Полный цикл: "сколько кальция мужчине 19-30?" → describe → query → ответ
#[tokio::test]
async fn test_full_roundtrip_calcium() {
    if !api_key_present() {
        eprintln!("SKIP: DEEPSEEK_API_KEY not set");
        return;
    }

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

/// Витамин C: DRI Vitamins — "сколько витамина C рекомендуется женщине 19-30 лет?"
#[tokio::test]
async fn test_roundtrip_vitamin_c() {
    if !api_key_present() {
        eprintln!("SKIP: DEEPSEEK_API_KEY not set");
        return;
    }

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

/// WHO Hb: "какие пороги анемии для беременных?"
#[tokio::test]
async fn test_roundtrip_who_hb() {
    if !api_key_present() {
        eprintln!("SKIP: DEEPSEEK_API_KEY not set");
        return;
    }

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

/// USDA Foods: "сколько кальция в молоке?"
#[tokio::test]
async fn test_roundtrip_usda_milk() {
    if !api_key_present() {
        eprintln!("SKIP: DEEPSEEK_API_KEY not set");
        return;
    }

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

/// Lab Ranges: "какие референсные значения гемоглобина в крови?"
#[tokio::test]
async fn test_roundtrip_lab_ranges() {
    if !api_key_present() {
        eprintln!("SKIP: DEEPSEEK_API_KEY not set");
        return;
    }

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
