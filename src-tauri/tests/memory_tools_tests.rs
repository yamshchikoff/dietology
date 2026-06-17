use dietology_lib::memory::conversational_preferences::PreferencesStore;
use dietology_lib::memory::facts::FactStore;
use dietology_lib::memory::findings::FindingStore;
use dietology_lib::memory::master_description::{LlmCredentials, MasterDescriptionStore};
use dietology_lib::memory::storage::MemoryStorage;
use dietology_lib::memory::tools::{
    register_memory_read_tools, register_memory_write_tools,
};
use dietology_lib::tools::registry::{ToolCall, ToolRegistry};
use serde_json::json;
use std::sync::Arc;

struct TestEnv {
    storage: Arc<MemoryStorage>,
    fact_store: Arc<FactStore>,
    finding_store: Arc<FindingStore>,
    master_store: Arc<MasterDescriptionStore>,
    prefs_store: Arc<PreferencesStore>,
    registry: ToolRegistry,
}

fn setup() -> TestEnv {
    let dir = tempfile::tempdir().unwrap();
    let storage = Arc::new(MemoryStorage::new(dir.path().to_path_buf()));
    let fact_store = Arc::new(FactStore::new(storage.clone()));
    let finding_store = Arc::new(FindingStore::new(storage.clone(), fact_store.clone()));
    let master_store = Arc::new(MasterDescriptionStore::new(storage.clone()));
    let prefs_store = Arc::new(PreferencesStore::new(storage.clone()));

    let mut registry = ToolRegistry::new();
    register_memory_read_tools(
        &mut registry,
        fact_store.clone(),
        finding_store.clone(),
        master_store.clone(),
        prefs_store.clone(),
    );
    let llm_creds = Arc::new(std::sync::Mutex::new(None::<LlmCredentials>));
    register_memory_write_tools(
        &mut registry,
        fact_store.clone(),
        finding_store.clone(),
        master_store.clone(),
        prefs_store.clone(),
        llm_creds,
    );

    TestEnv {
        storage,
        fact_store,
        finding_store,
        master_store,
        prefs_store,
        registry,
    }
}

fn dispatch(registry: &ToolRegistry, name: &str, args: serde_json::Value) -> serde_json::Value {
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: name.to_string(),
        arguments: args,
    };
    let result = registry.dispatch(&call).unwrap();
    serde_json::from_str(&result.content).unwrap()
}

fn dispatch_err(registry: &ToolRegistry, name: &str, args: serde_json::Value) -> String {
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: name.to_string(),
        arguments: args,
    };
    registry.dispatch(&call).unwrap_err()
}

fn dispatch_result(
    registry: &ToolRegistry,
    name: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: name.to_string(),
        arguments: args,
    };
    registry
        .dispatch(&call)
        .map(|r| serde_json::from_str(&r.content).unwrap())
}

// ── list_facts ─────────────────────────────────────────────────────────

#[test]
fn test_list_facts_empty() {
    let env = setup();
    let v = dispatch(&env.registry, "list_facts", json!({"offset": 0, "limit": 10}));
    assert!(v.as_array().unwrap().is_empty());
}

#[test]
fn test_list_facts_with_data() {
    let env = setup();
    env.fact_store
        .create_user_reported("Факт A", None)
        .unwrap();
    env.fact_store
        .create_user_reported("Факт B", None)
        .unwrap();

    let v = dispatch(&env.registry, "list_facts", json!({"offset": 0, "limit": 10}));
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["type"], "user_reported");
}

#[test]
fn test_list_facts_type_filter() {
    let env = setup();
    env.fact_store
        .create_user_reported("Факт", None)
        .unwrap();

    let v = dispatch(
        &env.registry,
        "list_facts",
        json!({"offset": 0, "limit": 10, "type": "user_reported"}),
    );
    assert_eq!(v.as_array().unwrap().len(), 1);

    let v2 = dispatch(
        &env.registry,
        "list_facts",
        json!({"offset": 0, "limit": 10, "type": "imported"}),
    );
    assert!(v2.as_array().unwrap().is_empty());
}

#[test]
fn test_list_facts_invalid_type() {
    let env = setup();
    let err = dispatch_err(
        &env.registry,
        "list_facts",
        json!({"offset": 0, "limit": 10, "type": "unknown"}),
    );
    assert!(err.contains("invalid fact type"));
}

#[test]
fn test_list_facts_pagination() {
    let env = setup();
    for i in 0..3 {
        env.fact_store
            .create_user_reported(&format!("Факт {i}"), None)
            .unwrap();
    }
    let page1 = dispatch(&env.registry, "list_facts", json!({"offset": 0, "limit": 2}));
    assert_eq!(page1.as_array().unwrap().len(), 2);
    let page2 = dispatch(&env.registry, "list_facts", json!({"offset": 2, "limit": 2}));
    assert_eq!(page2.as_array().unwrap().len(), 1);
}

// ── read_fact ──────────────────────────────────────────────────────────

#[test]
fn test_read_fact_user_reported() {
    let env = setup();
    let fact = env
        .fact_store
        .create_user_reported("Вес 80 кг", Some("2026-01-15"))
        .unwrap();

    let v = dispatch(
        &env.registry,
        "read_fact",
        json!({"fact_id": fact.id}),
    );
    // FactReadResult is tagged enum
    assert_eq!(v["fact_type"], "user_reported");
    assert_eq!(v["fact"]["content"], "Вес 80 кг");
    assert_eq!(v["fact"]["presumed_date"], "2026-01-15");
    assert!(v["fact"]["disclaimer"]
        .as_str()
        .unwrap()
        .contains("Данный факт сообщён пользователем"));
    assert!(v["versions"].as_array().unwrap().len() >= 1);
}

#[test]
fn test_read_fact_nonexistent() {
    let env = setup();
    let err = dispatch_err(
        &env.registry,
        "read_fact",
        json!({"fact_id": "urfact-nonexistent"}),
    );
    assert!(err.contains("not found") || err.contains("data file"));
}

// ── list_findings ──────────────────────────────────────────────────────

#[test]
fn test_list_findings_empty() {
    let env = setup();
    let v = dispatch(&env.registry, "list_findings", json!({"offset": 0, "limit": 10}));
    assert!(v.as_array().unwrap().is_empty());
}

#[test]
fn test_list_findings_with_data_and_pagination() {
    let env = setup();
    let fact = env.fact_store.create_user_reported("A", None).unwrap();
    env.finding_store
        .create("Вывод 1", &[fact.id.clone()], "r")
        .unwrap();
    env.finding_store
        .create("Вывод 2", &[fact.id], "r")
        .unwrap();

    let v = dispatch(&env.registry, "list_findings", json!({"offset": 0, "limit": 10}));
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert!(arr[0]["foundation_changed"].as_bool().is_some());
}
// ── read_finding ───────────────────────────────────────────────────────

#[test]
fn test_read_finding_exists() {
    let env = setup();
    let fact = env.fact_store.create_user_reported("A", None).unwrap();
    let finding = env
        .finding_store
        .create("Вывод", &[fact.id.clone()], "reason")
        .unwrap();

    let v = dispatch(
        &env.registry,
        "read_finding",
        json!({"finding_id": finding.id}),
    );
    assert_eq!(v["finding"]["content"], "Вывод");
    assert_eq!(v["finding"]["status"], "active");
    assert!(!v["based_on_facts"].as_array().unwrap().is_empty());
}

#[test]
fn test_read_finding_nonexistent() {
    let env = setup();
    let err = dispatch_err(
        &env.registry,
        "read_finding",
        json!({"finding_id": "finding-nonexistent"}),
    );
    assert!(err.contains("not found") || err.contains("data file"));
}

// ── read_master_description ────────────────────────────────────────────

#[test]
fn test_read_master_description_empty() {
    let env = setup();
    let err = dispatch_err(&env.registry, "read_master_description", json!({}));
    assert!(err.contains("not found") || err.contains("data file"));
}

// ── read_conversational_preferences ────────────────────────────────────

#[test]
fn test_read_conversational_preferences_empty() {
    let env = setup();
    let err = dispatch_err(
        &env.registry,
        "read_conversational_preferences",
        json!({}),
    );
    assert!(err.contains("not found") || err.contains("data file"));
}

// ── create_user_reported_fact ──────────────────────────────────────────

#[test]
fn test_create_user_reported_fact_success() {
    let env = setup();
    let v = dispatch(
        &env.registry,
        "create_user_reported_fact",
        json!({"content": "Вес 85 кг", "presumed_date": "2026-03-01"}),
    );
    assert_eq!(v["content"], "Вес 85 кг");
    assert_eq!(v["version"], 1);
    assert_eq!(v["author"], "agent");
    assert_eq!(v["presumed_author"], "user");
    assert!(v["disclaimer"].as_str().unwrap().contains("Данный факт сообщён"));
}

#[test]
fn test_create_user_reported_fact_oversized() {
    let env = setup();
    let long = "x".repeat(20000);
    let result = dispatch_result(
        &env.registry,
        "create_user_reported_fact",
        json!({"content": long}),
    );
    assert!(result.is_err(), "expected error for oversized fact content");
}

// ── correct_user_reported_fact ─────────────────────────────────────────

#[test]
fn test_correct_user_reported_fact_success() {
    let env = setup();
    let fact = env.fact_store.create_user_reported("Вес 85", None).unwrap();
    let v = dispatch(
        &env.registry,
        "correct_user_reported_fact",
        json!({
            "fact_id": fact.id,
            "content": "Вес 83",
            "reason": "уточнение"
        }),
    );
    assert_eq!(v["fact"]["version"], 2);
    assert_eq!(v["fact"]["content"], "Вес 83");
}

#[test]
fn test_correct_user_reported_fact_nonexistent() {
    let env = setup();
    let err = dispatch_err(
        &env.registry,
        "correct_user_reported_fact",
        json!({"fact_id": "urfact-nonexistent", "content": "x", "reason": "r"}),
    );
    assert!(err.contains("not found") || err.contains("data file"));
}

// ── create_finding ─────────────────────────────────────────────────────

#[test]
fn test_create_finding_success() {
    let env = setup();
    let fact = env.fact_store.create_user_reported("Тест", None).unwrap();
    let v = dispatch(
        &env.registry,
        "create_finding",
        json!({
            "content": "Низкий уровень железа в рационе",
            "based_on_fact_ids": [fact.id],
            "reason": "анализ"
        }),
    );
    assert_eq!(v["content"], "Низкий уровень железа в рационе");
    assert_eq!(v["status"], "active");
    assert!(!v["foundation_changed"].as_bool().unwrap());
}

#[test]
fn test_create_finding_invalid_based_on() {
    let env = setup();
    let err = dispatch_err(
        &env.registry,
        "create_finding",
        json!({
            "content": "Вывод",
            "based_on_fact_ids": ["urfact-nonexistent"],
            "reason": "r"
        }),
    );
    assert!(err.contains("not found"));
}

#[test]
fn test_create_finding_oversized() {
    let env = setup();
    let fact = env.fact_store.create_user_reported("X", None).unwrap();
    let long = "x".repeat(20000);
    let result = dispatch_result(
        &env.registry,
        "create_finding",
        json!({"content": long, "based_on_fact_ids": [fact.id], "reason": "r"}),
    );
    assert!(result.is_err(), "expected error for oversized finding content");
}

// ── resolve_finding_status ─────────────────────────────────────────────

#[test]
fn test_resolve_superseded() {
    let env = setup();
    let fact = env.fact_store.create_user_reported("A", None).unwrap();
    let finding = env.finding_store.create("B", &[fact.id], "r").unwrap();
    let v = dispatch(
        &env.registry,
        "resolve_finding_status",
        json!({"finding_id": finding.id, "status": "superseded", "reason": "устарело"}),
    );
    assert_eq!(v["status"], "superseded");
}

#[test]
fn test_resolve_superseded_twice_fails() {
    let env = setup();
    let fact = env.fact_store.create_user_reported("A", None).unwrap();
    let finding = env.finding_store.create("B", &[fact.id], "r").unwrap();
    dispatch(
        &env.registry,
        "resolve_finding_status",
        json!({"finding_id": finding.id, "status": "superseded", "reason": "first"}),
    );
    let err = dispatch_err(
        &env.registry,
        "resolve_finding_status",
        json!({"finding_id": finding.id, "status": "superseded", "reason": "second"}),
    );
    assert!(err.contains("already superseded"));
}

#[test]
fn test_reaffirmed_without_foundation_changed_fails() {
    let env = setup();
    let fact = env.fact_store.create_user_reported("A", None).unwrap();
    let finding = env.finding_store.create("B", &[fact.id], "r").unwrap();
    let err = dispatch_err(
        &env.registry,
        "resolve_finding_status",
        json!({"finding_id": finding.id, "status": "reaffirmed", "reason": "x"}),
    );
    assert!(err.contains("foundation_changed"));
}

// ── rewrite_master_description ─────────────────────────────────────────

#[test]
fn test_rewrite_master_description_success() {
    let env = setup();
    let v = dispatch(
        &env.registry,
        "rewrite_master_description",
        json!({
            "content": "Целостный профиль",
            "based_on_facts": [],
            "based_on_findings": []
        }),
    );
    assert_eq!(v["version"], 1);

    // Verify it's readable
    let read = dispatch(&env.registry, "read_master_description", json!({}));
    assert_eq!(read["master_description"]["content"], "Целостный профиль");
}

#[test]
fn test_rewrite_master_description_oversized() {
    let env = setup();
    let long = "x".repeat(1000000);
    let result = dispatch_result(
        &env.registry,
        "rewrite_master_description",
        json!({"content": long, "based_on_facts": [], "based_on_findings": []}),
    );
    assert!(result.is_err(), "expected error for oversized master description");
}

// ── rewrite_conversational_preferences ─────────────────────────────────

#[test]
fn test_rewrite_conversational_preferences_success() {
    let env = setup();
    let v = dispatch(
        &env.registry,
        "rewrite_conversational_preferences",
        json!({"content": "Отвечай кратко"}),
    );
    assert_eq!(v["status"], "ok");

    let read = dispatch(
        &env.registry,
        "read_conversational_preferences",
        json!({}),
    );
    assert_eq!(read["content"], "Отвечай кратко");
}

#[test]
fn test_rewrite_conversational_preferences_oversized() {
    let env = setup();
    let long = "x".repeat(20000);
    let err = dispatch_err(
        &env.registry,
        "rewrite_conversational_preferences",
        json!({"content": long}),
    );
    assert!(err.contains("exceed token limit"));
}

// ── restore_conversational_preferences ─────────────────────────────────

#[test]
fn test_restore_conversational_preferences() {
    let env = setup();
    dispatch(
        &env.registry,
        "rewrite_conversational_preferences",
        json!({"content": "v1"}),
    );
    dispatch(
        &env.registry,
        "rewrite_conversational_preferences",
        json!({"content": "v2"}),
    );

    let v = dispatch(
        &env.registry,
        "restore_conversational_preferences",
        json!({}),
    );
    assert_eq!(v["status"], "ok");

    let read = dispatch(
        &env.registry,
        "read_conversational_preferences",
        json!({}),
    );
    assert_eq!(read["content"], "v1");
}

#[test]
fn test_restore_no_backup_fails() {
    let env = setup();
    let err = dispatch_err(
        &env.registry,
        "restore_conversational_preferences",
        json!({}),
    );
    assert!(err.contains("backup") || err.contains("no backup"));
}

// ── run_subagent (wiremock) ────────────────────────────────────────────

#[tokio::test]
async fn test_run_subagent_updates_master_description() {
    use dietology_lib::memory::tools::run_subagent;
    use std::sync::Mutex;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    let dir = tempfile::tempdir().unwrap();
    let storage = Arc::new(MemoryStorage::new(dir.path().to_path_buf()));
    let fact_store = Arc::new(FactStore::new(storage.clone()));
    let finding_store = Arc::new(FindingStore::new(storage.clone(), fact_store.clone()));
    let master_store = Arc::new(MasterDescriptionStore::new(storage.clone()));

    let fact = fact_store.create_user_reported("Факт 1", None).unwrap();
    let finding = finding_store
        .create("Находка 1", &[fact.id.clone()], "reason")
        .unwrap();

    let server = MockServer::start().await;

    let call_count = Arc::new(Mutex::new(0u32));
    let count = call_count.clone();
    let fact_id = fact.id.clone();
    let finding_id = finding.id.clone();

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(move |_req: &wiremock::Request| {
            let mut n = count.lock().unwrap();
            *n += 1;
            match *n {
                1 => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "msg_001",
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "tool_use", "id": "tu_1", "name": "read_master_description", "input": {}},
                        {"type": "tool_use", "id": "tu_2", "name": "list_facts", "input": {"offset": 0, "limit": 50}},
                        {"type": "tool_use", "id": "tu_3", "name": "list_findings", "input": {"offset": 0, "limit": 50}}
                    ],
                    "stop_reason": "tool_use",
                    "usage": {"input_tokens": 100, "output_tokens": 50}
                })),
                2 => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "msg_002",
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "tool_use", "id": "tu_rw", "name": "rewrite_master_description", "input": {
                            "content": "Тестовый профиль на основе фактов",
                            "based_on_facts": [fact_id.clone()],
                            "based_on_findings": [finding_id.clone()]
                        }}
                    ],
                    "stop_reason": "tool_use",
                    "usage": {"input_tokens": 200, "output_tokens": 100}
                })),
                _ => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "msg_003",
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Готово."}],
                    "stop_reason": "end_turn",
                    "usage": {"input_tokens": 300, "output_tokens": 20}
                })),
            }
        })
        .mount(&server)
        .await;

    let server_uri = server.uri();

    // Run subagent on a dedicated OS thread — no tokio runtime on that
    // thread, so run_subagent creates its own. Avoids nested block_on.
    let store_ref = master_store.clone();
    let result = tokio::task::spawn_blocking(move || {
        let llm_creds = LlmCredentials {
            api_key: "test-key".into(),
            api_base_url: server_uri,
            model: "test-model".into(),
        };
        run_subagent(fact_store, finding_store, store_ref, &llm_creds)
    })
    .await
    .unwrap();

    assert!(result.is_ok(), "run_subagent failed: {:?}", result.err());
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["version"], 1);

    let (doc, fc) = master_store.read().unwrap();
    assert_eq!(doc.content, "Тестовый профиль на основе фактов");
    assert!(!fc);
}

