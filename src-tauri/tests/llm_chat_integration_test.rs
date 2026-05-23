use dietology_lib::data::DataLoader;
use dietology_lib::llm::client::LlmClient;
use dietology_lib::llm::types::*;
use dietology_lib::tools::registry::ToolRegistry;
use std::sync::Arc;

/// Полный цикл: "сколько кальция мужчине 19-30?" → describe → query → ответ
///
/// Требует DEEPSEEK_API_KEY в окружении.
/// Если ключ не задан — тест игнорируется.
#[tokio::test]
async fn test_full_roundtrip_calcium() {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("SKIP: DEEPSEEK_API_KEY not set");
        return;
    }

    let loader = DataLoader::for_development();
    let mut registry = ToolRegistry::new();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);
    dietology_lib::tools::query::register_query_tools(&mut registry, &loader);

    let client = LlmClient::new(Arc::new(registry)).expect("failed to create client");

    let system_prompt = "\
Ты — ассистент по питанию. Отвечай на русском языке.
Для поиска данных используй инструменты: сначала describe для навигации, потом query для конкретных значений.";

    let mut messages = vec![Message {
        role: "user".into(),
        content: vec![ContentBlock::Text {
            text: "Сколько кальция рекомендуется мужчине 19-30 лет?".into(),
        }],
    }];

    let response = client.chat(&mut messages, system_prompt).await;

    match response {
        Ok(resp) => {
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
            assert!(has_tool_use, "expected at least one tool_use in message history");

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
        Err(e) => {
            panic!("chat() failed: {e}");
        }
    }
}
