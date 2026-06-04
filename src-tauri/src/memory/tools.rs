use std::sync::Arc;

use serde_json::json;

use crate::tools::registry::ToolRegistry;

use super::conversational_preferences::PreferencesStore;
use super::facts::FactStore;
use super::findings::FindingStore;
use super::macro_conclusion::{LlmCredentials, MacroConclusionStore};
use super::types::{FactType, FindingStatus};

pub fn register_memory_read_tools(
    registry: &mut ToolRegistry,
    fact_store: Arc<FactStore>,
    finding_store: Arc<FindingStore>,
    macro_store: Arc<MacroConclusionStore>,
    prefs_store: Arc<PreferencesStore>,
) {
    let fs = fact_store.clone();
    registry.register(
        "list_facts",
        "List facts with pagination. Returns summaries: id, type, title (first 80 chars), created_at, version, findings_count.",
        json!({
            "type": "object",
            "properties": {
                "type": { "type": "string", "enum": ["imported", "user_reported"] },
                "offset": { "type": "integer", "minimum": 0 },
                "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
            },
            "required": ["offset", "limit"]
        }),
        Box::new(move |args: &serde_json::Value| -> Result<String, String> {
            let fact_type = args.get("type").and_then(|v| v.as_str()).map(|s| match s {
                "imported" => FactType::Imported,
                "user_reported" => FactType::UserReported,
                _ => FactType::UserReported,
            });
            let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as u32;
            let result = fs.list(fact_type, offset, limit)
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        }),
    );

    let fs2 = fact_store;
    registry.register(
        "read_fact",
        "Read a single fact in full. Returns current version, versions index, and linked findings. For user-reported facts, includes mandatory disclaimer.",
        json!({
            "type": "object",
            "properties": {
                "fact_id": { "type": "string" }
            },
            "required": ["fact_id"]
        }),
        Box::new(move |args: &serde_json::Value| -> Result<String, String> {
            let fact_id = args.get("fact_id").and_then(|v| v.as_str()).ok_or("missing fact_id")?;
            let result = fs2.read(fact_id).map_err(|e| e.to_string())?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        }),
    );

    let fstore = finding_store.clone();
    registry.register(
        "list_findings",
        "List findings with pagination. Returns summaries: id, title (first 80 chars), created_at, status (active|superseded), foundation_changed flag, based_on_fact_ids.",
        json!({
            "type": "object",
            "properties": {
                "offset": { "type": "integer", "minimum": 0 },
                "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
            },
            "required": ["offset", "limit"]
        }),
        Box::new(move |args: &serde_json::Value| -> Result<String, String> {
            let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as u32;
            let result = fstore.list(offset, limit).map_err(|e| e.to_string())?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        }),
    );

    let fstore2 = finding_store;
    registry.register(
        "read_finding",
        "Read a single finding in full. Returns finding content, status, foundation_changed flag, and summaries of all based-on facts.",
        json!({
            "type": "object",
            "properties": {
                "finding_id": { "type": "string" }
            },
            "required": ["finding_id"]
        }),
        Box::new(move |args: &serde_json::Value| -> Result<String, String> {
            let finding_id = args.get("finding_id").and_then(|v| v.as_str()).ok_or("missing finding_id")?;
            let result = fstore2.read(finding_id).map_err(|e| e.to_string())?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        }),
    );

    let ms = macro_store.clone();
    registry.register(
        "read_macro_conclusion",
        "Read the current macro-conclusion (master description). Returns the full document plus foundation_changed flag. Macro-conclusion is the model's holistic narrative about the user, synthesizing all facts and findings.",
        json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
            let (doc, fc) = ms.read().map_err(|e| e.to_string())?;
            serde_json::to_string(&json!({
                "macro_conclusion": doc,
                "foundation_changed": fc
            }))
            .map_err(|e| e.to_string())
        }),
    );

    let ps = prefs_store;
    registry.register(
        "read_conversational_preferences",
        "Read current conversational preferences — how the user prefers to communicate.",
        json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
            let prefs = ps.read().map_err(|e| e.to_string())?;
            serde_json::to_string(&prefs).map_err(|e| e.to_string())
        }),
    );
}

pub fn register_memory_write_tools(
    registry: &mut ToolRegistry,
    fact_store: Arc<FactStore>,
    finding_store: Arc<FindingStore>,
    macro_store: Arc<MacroConclusionStore>,
    prefs_store: Arc<PreferencesStore>,
    llm_creds: Arc<std::sync::Mutex<Option<LlmCredentials>>>,
) {
    let fs = fact_store.clone();
    registry.register(
        "create_user_reported_fact",
        "Record a fact reported by the user about themselves. Core injects mandatory disclaimer with current date. Max 1024 tokens total (disclaimer + content). Use presumed_date when user indicates when the event occurred.",
        json!({
            "type": "object",
            "properties": {
                "content": { "type": "string", "description": "Fact content as reported by user" },
                "presumed_date": { "type": "string", "description": "ISO 8601 date or free text like 'last summer', 'in 2020' — when the user says the event occurred" }
            },
            "required": ["content"]
        }),
        Box::new(move |args: &serde_json::Value| -> Result<String, String> {
            let content = args.get("content").and_then(|v| v.as_str()).ok_or("missing content")?;
            let presumed_date = args.get("presumed_date").and_then(|v| v.as_str());
            let fact = fs.create_user_reported(content, presumed_date)
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&fact).map_err(|e| e.to_string())
        }),
    );

    let fs2 = fact_store.clone();
    let fstore2 = finding_store.clone();
    let ms = macro_store.clone();
    registry.register(
        "correct_user_reported_fact",
        "Correct a previously recorded user-reported fact. Saves old version, creates new version. Sets foundation_changed=true on all dependent findings and macro-conclusion. Requires fact_id, new content, and reason for correction.",
        json!({
            "type": "object",
            "properties": {
                "fact_id": { "type": "string" },
                "content": { "type": "string" },
                "reason": { "type": "string" },
                "presumed_date": { "type": "string" }
            },
            "required": ["fact_id", "content", "reason"]
        }),
        Box::new(move |args: &serde_json::Value| -> Result<String, String> {
            let fact_id = args.get("fact_id").and_then(|v| v.as_str()).ok_or("missing fact_id")?;
            let content = args.get("content").and_then(|v| v.as_str()).ok_or("missing content")?;
            let reason = args.get("reason").and_then(|v| v.as_str()).ok_or("missing reason")?;
            let presumed_date = args.get("presumed_date").and_then(|v| v.as_str());
            let corrected = fs2.correct(fact_id, content, reason, presumed_date)
                .map_err(|e| e.to_string())?;
            let affected = fstore2.mark_foundation_changed(fact_id)
                .map_err(|e| e.to_string())?;
            let _ = ms.mark_foundation_changed();
            serde_json::to_string(&json!({
                "fact": corrected,
                "affected_findings": affected
            }))
            .map_err(|e| e.to_string())
        }),
    );

    let fstore2 = finding_store.clone();
    registry.register(
        "create_finding",
        "Create a new finding (insight). Findings are immutable — once written, never modified. Max 1024 tokens. Must reference facts via based_on_fact_ids. Core validates all referenced facts exist and updates bidirectional links atomically.",
        json!({
            "type": "object",
            "properties": {
                "content": { "type": "string", "description": "Finding content — full conclusion, not a delta" },
                "based_on_fact_ids": { "type": "array", "items": { "type": "string" }, "description": "Fact IDs this finding is based on" },
                "reason": { "type": "string", "description": "Why this finding was created — context, not versioning" }
            },
            "required": ["content", "based_on_fact_ids", "reason"]
        }),
        Box::new(move |args: &serde_json::Value| -> Result<String, String> {
            let content = args.get("content").and_then(|v| v.as_str()).ok_or("missing content")?;
            let based_on: Vec<String> = args.get("based_on_fact_ids")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .ok_or("missing based_on_fact_ids")?;
            let reason = args.get("reason").and_then(|v| v.as_str()).ok_or("missing reason")?;
            let finding = fstore2.create(content, &based_on, reason)
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&finding).map_err(|e| e.to_string())
        }),
    );

    let fstore3 = finding_store.clone();
    registry.register(
        "resolve_finding_status",
        "Resolve a finding's status. 'superseded' marks it obsolete. 'reaffirmed' clears foundation_changed flag — only valid when foundation_changed=true. Core rejects invalid state transitions.",
        json!({
            "type": "object",
            "properties": {
                "finding_id": { "type": "string" },
                "status": { "type": "string", "enum": ["superseded", "reaffirmed"] },
                "reason": { "type": "string" }
            },
            "required": ["finding_id", "status", "reason"]
        }),
        Box::new(move |args: &serde_json::Value| -> Result<String, String> {
            let finding_id = args.get("finding_id").and_then(|v| v.as_str()).ok_or("missing finding_id")?;
            let status_str = args.get("status").and_then(|v| v.as_str()).ok_or("missing status")?;
            let reason = args.get("reason").and_then(|v| v.as_str()).ok_or("missing reason")?;
            let status = match status_str {
                "superseded" => FindingStatus::Superseded,
                "reaffirmed" => FindingStatus::Active,
                _ => return Err(format!("invalid status: {status_str}")),
            };
            let finding = fstore3.resolve_status(finding_id, status, reason)
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&finding).map_err(|e| e.to_string())
        }),
    );

    let ms2 = macro_store.clone();
    registry.register(
        "rewrite_macro_conclusion",
        "Directly rewrite the macro-conclusion (master description). Max 50000 tokens. Old version is preserved, new version becomes active. Use this for quick updates. For a full review against all facts/findings, use update_macro_conclusion instead.",
        json!({
            "type": "object",
            "properties": {
                "content": { "type": "string", "description": "Full text of the new macro-conclusion" }
            },
            "required": ["content"]
        }),
        Box::new(move |args: &serde_json::Value| -> Result<String, String> {
            let content = args.get("content").and_then(|v| v.as_str()).ok_or("missing content")?;
            let v = ms2.rewrite(content, vec![], vec![])
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&json!({"version": v})).map_err(|e| e.to_string())
        }),
    );

    let ms3 = macro_store.clone();
    let fs3 = fact_store.clone();
    let fstore4 = finding_store.clone();
    let creds = llm_creds.clone();
    registry.register(
        "update_macro_conclusion",
        "Update macro-conclusion via subagent. The subagent loads current macro-conclusion, all facts, all findings, checks consistency, and rewrites. Call this during housekeeping. No parameters needed — subagent traverses all data automatically.",
        json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
            let guard = creds.lock().map_err(|e| e.to_string())?;
            let c = guard.as_ref().ok_or_else(|| "API key not set — use set_key first".to_string())?;
            run_subagent(fs3.clone(), fstore4.clone(), ms3.clone(), c)
        }),
    );

    let ps2 = prefs_store.clone();
    registry.register(
        "rewrite_conversational_preferences",
        "Rewrite conversational preferences entirely. Max 1024 tokens. Previous version is saved as a single backup (one restore available). This is a full replacement, not an incremental update.",
        json!({
            "type": "object",
            "properties": {
                "content": { "type": "string" }
            },
            "required": ["content"]
        }),
        Box::new(move |args: &serde_json::Value| -> Result<String, String> {
            let content = args.get("content").and_then(|v| v.as_str()).ok_or("missing content")?;
            ps2.rewrite(content).map_err(|e| e.to_string())?;
            Ok(json!({"status": "ok"}).to_string())
        }),
    );

    let ps3 = prefs_store;
    registry.register(
        "restore_conversational_preferences",
        "Restore conversational preferences from backup. Swaps current and backup. Calling again reverses the restore.",
        json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
            ps3.restore().map_err(|e| e.to_string())?;
            Ok(json!({"status": "ok"}).to_string())
        }),
    );
}

fn run_subagent(
    fact_store: Arc<FactStore>,
    finding_store: Arc<FindingStore>,
    macro_store: Arc<MacroConclusionStore>,
    llm_creds: &LlmCredentials,
) -> Result<String, String> {
    use crate::llm::client::LlmClient;
    use crate::llm::types::{ContentBlock, Message};

    let subagent_registry = build_subagent_registry(
        fact_store.clone(),
        finding_store.clone(),
        macro_store.clone(),
    );
    let subagent_client = LlmClient::with_credentials(
        Arc::new(subagent_registry),
        llm_creds.api_key.clone(),
        llm_creds.api_base_url.clone(),
        llm_creds.model.clone(),
    )
    .map_err(|e| format!("failed to create subagent client: {e}"))?;

    let subagent_prompt = "\
Ты — вспомогательный агент. Твоя задача: переписать macro-conclusion (мастер-описание пользователя).

Выполни СТРОГО ПОСЛЕДОВАТЕЛЬНО:
1. Вызови read_macro_conclusion чтобы получить текущую версию (может отсутствовать — тогда начни с нуля).
2. Вызови list_facts и пагинируй через все факты (list + read каждый). БЕЗ ИСКЛЮЧЕНИЙ — все user-reported и все imported факты.
3. Вызови list_findings и пагинируй через все findings (list + read каждый). БЕЗ ИСКЛЮЧЕНИЙ.
4. Проверь консистентность:
   - Есть ли факты без связанных findings? Если да — отметь это.
   - Есть ли findings с foundation_changed=true без разрешения? Если да — отметь.
   - Есть ли расхождения между фактами и macro-conclusion?
5. Вызови rewrite_macro_conclusion(content) с ПОЛНЫМ текстом нового macro-conclusion (≤50000 токенов). Заполни поля based_on_facts (ВСЕ факты) и based_on_findings (ВСЕ findings).

Macro-conclusion — это целостный рассказ о пользователе: что известно, ключевые выводы, критические данные (аллергии, противопоказания), текущая картина здоровья и питания. Это НЕ таблица и НЕ список — это связный текст, синтезирующий все данные.

НЕ вызывай create_finding, correct_user_reported_fact и другие write-тулы. ТОЛЬКО rewrite_macro_conclusion.";

    let handle = tokio::runtime::Handle::current();
    let version_before = macro_store
        .read_optional()
        .ok()
        .flatten()
        .map(|(_, _fc)| 0u64)
        .unwrap_or(0);

    let result = handle.block_on(async {
        let mut messages = vec![Message {
            role: "user".into(),
            content: vec![ContentBlock::Text {
                text: "Перепиши macro-conclusion на основе всех фактов и находок.".into(),
            }],
        }];
        subagent_client.chat(&mut messages, subagent_prompt).await
    });

    match result {
        Ok(response) => {
            let version_after = macro_store
                .read_optional()
                .map_err(|e| e.to_string())?
                .map(|(doc, _)| doc.version)
                .unwrap_or(0);

            if version_after <= version_before {
                return Err(
                    "subagent did not call rewrite_macro_conclusion (version unchanged)".into()
                );
            }

            let (doc, _) = macro_store.read().map_err(|e| e.to_string())?;
            if doc.based_on_facts.is_empty() {
                return Err("subagent wrote empty based_on_facts".into());
            }

            Ok(json!({
                "status": "ok",
                "version": doc.version,
                "based_on_facts_count": doc.based_on_facts.len(),
                "based_on_findings_count": doc.based_on_findings.len(),
                "subagent_response": response.final_text
            })
            .to_string())
        }
        Err(e) => Err(format!("subagent LLM call failed: {e}")),
    }
}

fn build_subagent_registry(
    fact_store: Arc<FactStore>,
    finding_store: Arc<FindingStore>,
    macro_store: Arc<MacroConclusionStore>,
) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    let fs = fact_store.clone();
    let fs2 = fact_store.clone();
    let fstore = finding_store.clone();
    let fstore2 = finding_store.clone();
    let ms = macro_store.clone();
    let ms2 = macro_store.clone();

    // The subagent gets only 6 tools: read tools + rewrite_macro_conclusion.

    // list_facts
    {
        let fs = fs.clone();
        registry.register(
            "list_facts",
            "List facts with pagination.",
            json!({
                "type": "object",
                "properties": {
                    "type": { "type": "string", "enum": ["imported", "user_reported"] },
                    "offset": { "type": "integer", "minimum": 0 },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                },
                "required": ["offset", "limit"]
            }),
            Box::new(move |args: &serde_json::Value| -> Result<String, String> {
                let fact_type = args.get("type").and_then(|v| v.as_str()).map(|s| match s {
                    "imported" => FactType::Imported,
                    "user_reported" => FactType::UserReported,
                    _ => FactType::UserReported,
                });
                let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as u32;
                fs.list(fact_type, offset, limit).map(|r| serde_json::to_string(&r).unwrap())
                    .map_err(|e| e.to_string())
            }),
        );
    }

    // read_fact
    {
        let fs = fs2;
        registry.register(
            "read_fact",
            "Read a single fact in full.",
            json!({
                "type": "object",
                "properties": { "fact_id": { "type": "string" } },
                "required": ["fact_id"]
            }),
            Box::new(move |args: &serde_json::Value| -> Result<String, String> {
                let fact_id = args.get("fact_id").and_then(|v| v.as_str()).ok_or("missing fact_id")?;
                fs.read(fact_id).map(|r| serde_json::to_string(&r).unwrap())
                    .map_err(|e| e.to_string())
            }),
        );
    }

    // list_findings
    {
        let fstore = fstore;
        registry.register(
            "list_findings",
            "List findings with pagination.",
            json!({
                "type": "object",
                "properties": {
                    "offset": { "type": "integer", "minimum": 0 },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                },
                "required": ["offset", "limit"]
            }),
            Box::new(move |args: &serde_json::Value| -> Result<String, String> {
                let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as u32;
                fstore.list(offset, limit).map(|r| serde_json::to_string(&r).unwrap())
                    .map_err(|e| e.to_string())
            }),
        );
    }

    // read_finding
    {
        let fstore = fstore2;
        registry.register(
            "read_finding",
            "Read a single finding in full.",
            json!({
                "type": "object",
                "properties": { "finding_id": { "type": "string" } },
                "required": ["finding_id"]
            }),
            Box::new(move |args: &serde_json::Value| -> Result<String, String> {
                let finding_id = args.get("finding_id").and_then(|v| v.as_str()).ok_or("missing finding_id")?;
                fstore.read(finding_id).map(|r| serde_json::to_string(&r).unwrap())
                    .map_err(|e| e.to_string())
            }),
        );
    }

    // read_macro_conclusion
    {
        let ms = ms;
        registry.register(
            "read_macro_conclusion",
            "Read the current macro-conclusion.",
            json!({ "type": "object", "properties": {}, "required": [] }),
            Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
                let (doc, fc) = ms.read().map_err(|e| e.to_string())?;
                serde_json::to_string(&json!({"macro_conclusion": doc, "foundation_changed": fc}))
                    .map_err(|e| e.to_string())
            }),
        );
    }

    // rewrite_macro_conclusion
    {
        let ms = ms2;
        registry.register(
            "rewrite_macro_conclusion",
            "Rewrite macro-conclusion with full text. Max 50000 tokens.",
            json!({
                "type": "object",
                "properties": { "content": { "type": "string" } },
                "required": ["content"]
            }),
            Box::new(move |args: &serde_json::Value| -> Result<String, String> {
                let content = args.get("content").and_then(|v| v.as_str()).ok_or("missing content")?;
                ms.rewrite(content, vec![], vec![])
                    .map(|v| json!({"version": v}).to_string())
                    .map_err(|e| e.to_string())
            }),
        );
    }

    registry
}
