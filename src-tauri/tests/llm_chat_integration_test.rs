mod common;
use common::*;

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
