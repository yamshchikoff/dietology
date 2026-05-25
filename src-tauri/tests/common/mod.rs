#![allow(dead_code)]

use dietology_lib::data::DataLoader;
use dietology_lib::llm::client::LlmClient;
use dietology_lib::llm::types::*;
use dietology_lib::tools::registry::ToolRegistry;
use std::sync::Arc;

pub fn setup_client() -> Result<(LlmClient, String), LlmError> {
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

pub fn user_message(text: &str) -> Message {
    Message {
        role: "user".into(),
        content: vec![ContentBlock::Text {
            text: text.into(),
        }],
    }
}

pub fn assert_valid_response(resp: &LlmResponse, messages: &[Message]) {
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

pub fn resolve_api_key() -> Option<String> {
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
