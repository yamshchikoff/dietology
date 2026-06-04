use std::sync::Arc;

use crate::error::{AppError, AppResult};

use super::git::auto_commit;
use super::storage::MemoryStorage;
use super::types::{
    FactReadResult, FactSummary, FactType, FactVersion, ImportedFact, UserReportedFact,
};

const MAX_USER_REPORTED_TOKENS: usize = 1024;

static DISCLAIMER_TEMPLATE: &str = "Данный факт сообщён пользователем в диалоге <ДАТА ФИКСАЦИИ>. Это дата фиксации сообщения моделью, а не время обсуждаемого события. Данный факт является суждением пользователя и получен в диалоге с ним — он может быть передан неточно, ошибочно или ложно. Загруженные в систему медицинские исследования обладают безусловным приоритетом над данным фактом. При выявлении расхождений с импортированными исследованиями причину расхождения следует выяснить в разговоре с пользователем и отметить отдельным суждением в разделе findings.";

pub(crate) fn generate_id(prefix: &str) -> String {
    generate_id_impl(prefix)
}

pub(crate) fn generate_finding_id() -> String {
    generate_id_impl("finding")
}

fn generate_id_impl(prefix: &str) -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let ts = dur.as_nanos() as u64;
    let random: u32 = (ts as u32).wrapping_mul(1103515245).wrapping_add(12345);
    format!("{prefix}-{ts:x}-{random:x}")
}

fn build_disclaimer() -> String {
    let date = MemoryStorage::now_iso();
    DISCLAIMER_TEMPLATE.replace("<ДАТА ФИКСАЦИИ>", &date)
}

pub struct FactStore {
    storage: Arc<MemoryStorage>,
}

impl FactStore {
    pub fn new(storage: Arc<MemoryStorage>) -> Self {
        Self { storage }
    }

    pub fn create_user_reported(
        &self,
        content: &str,
        presumed_date: Option<&str>,
    ) -> AppResult<UserReportedFact> {
        let disclaimer = build_disclaimer();
        let combined = format!("{disclaimer}\n\n{content}");
        let tokens = MemoryStorage::estimate_tokens(&combined);

        if tokens > MAX_USER_REPORTED_TOKENS {
            return Err(AppError::Io(format!(
                "content exceeds token limit: {tokens}/{MAX_USER_REPORTED_TOKENS} tokens"
            )));
        }

        let fact_id = generate_id("urfact");
        let now = MemoryStorage::now_iso();

        let fact = UserReportedFact {
            id: fact_id.clone(),
            fact_type: FactType::UserReported,
            version: 1,
            created_at: now.clone(),
            author: "agent".into(),
            presumed_date: presumed_date.map(|s| s.to_string()),
            presumed_author: "user".into(),
            content: content.to_string(),
            disclaimer,
            findings: Vec::new(),
        };

        let dir = format!("facts/user-reported/{}", fact_id);
        let fact_json = serde_json::to_string_pretty(&fact)?;

        let versions = vec![FactVersion {
            version: 1,
            created_at: now,
            reason: "initial creation".into(),
        }];
        let versions_json = serde_json::to_string_pretty(&versions)?;

        self.storage
            .atomic_write(&format!("{dir}/v1.json"), &fact_json)?;
        self.storage
            .atomic_write(&format!("{dir}/versions.json"), &versions_json)?;

        let _ = auto_commit(
            &self.storage,
            &format!("memory: create user-reported fact {fact_id}"),
            &[&format!("{dir}/v1.json"), &format!("{dir}/versions.json")],
        );

        Ok(fact)
    }

    pub fn read(&self, fact_id: &str) -> AppResult<FactReadResult> {
        if fact_id.starts_with("urfact-") {
            self.read_user_reported(fact_id)
        } else if fact_id.starts_with("ifact-") {
            self.read_imported(fact_id)
        } else {
            Err(AppError::DataFileNotFound(format!(
                "unknown fact id prefix: {fact_id}"
            )))
        }
    }

    fn read_user_reported(&self, fact_id: &str) -> AppResult<FactReadResult> {
        let dir = format!("facts/user-reported/{fact_id}");
        let versions: Vec<FactVersion> = self
            .storage
            .read_json(&format!("{dir}/versions.json"))?;

        let current_version = versions.last().map(|v| v.version).unwrap_or(1);
        let fact: UserReportedFact = self
            .storage
            .read_json(&format!("{dir}/v{current_version}.json"))?;

        let finding_summaries = self.read_finding_summaries(&fact.findings);

        Ok(FactReadResult::UserReported {
            fact,
            versions,
            findings: finding_summaries,
        })
    }

    pub fn read_imported(&self, fact_id: &str) -> AppResult<FactReadResult> {
        let dir = format!("facts/imported/{fact_id}");
        let versions: Vec<FactVersion> = self
            .storage
            .read_json(&format!("{dir}/versions.json"))?;

        let current_version = versions.last().map(|v| v.version).unwrap_or(1);
        let fact: ImportedFact = self
            .storage
            .read_json(&format!("{dir}/v{current_version}.json"))?;

        let finding_summaries = self.read_finding_summaries(&fact.findings);

        Ok(FactReadResult::Imported {
            fact,
            versions,
            findings: finding_summaries,
        })
    }

    fn read_finding_summaries(&self, finding_ids: &[String]) -> Vec<super::types::FindingSummary> {
        finding_ids
            .iter()
            .filter_map(|fid| {
                let finding: Option<super::types::Finding> = self
                    .storage
                    .read_json_optional(&format!("findings/{fid}/finding.json"))
                    .ok()
                    .flatten();
                finding.map(|f| super::types::FindingSummary {
                    id: f.id,
                    title: f.content.chars().take(80).collect(),
                    created_at: f.created_at,
                    status: f.status,
                    foundation_changed: f.foundation_changed,
                    based_on_fact_ids: f.based_on,
                })
            })
            .collect()
    }

    pub fn list(
        &self,
        fact_type: Option<FactType>,
        offset: u32,
        limit: u32,
    ) -> AppResult<Vec<FactSummary>> {
        let mut summaries = Vec::new();

        let need_user = fact_type.is_none() || fact_type == Some(FactType::UserReported);
        let need_imported = fact_type.is_none() || fact_type == Some(FactType::Imported);

        if need_user {
            let entries = self
                .storage
                .read_dir_entries("facts/user-reported")
                .unwrap_or_default();
            for entry in &entries {
                if let Some(summary) = self.build_fact_summary("facts/user-reported", entry, FactType::UserReported) {
                    summaries.push(summary);
                }
            }
        }

        if need_imported {
            let entries = self
                .storage
                .read_dir_entries("facts/imported")
                .unwrap_or_default();
            for entry in &entries {
                if let Some(summary) = self.build_fact_summary("facts/imported", entry, FactType::Imported) {
                    summaries.push(summary);
                }
            }
        }

        summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let start = offset as usize;
        let end = std::cmp::min(start + limit as usize, summaries.len());
        if start >= summaries.len() {
            return Ok(Vec::new());
        }
        Ok(summaries[start..end].to_vec())
    }

    fn build_fact_summary(
        &self,
        base_dir: &str,
        entry: &str,
        fact_type: FactType,
    ) -> Option<FactSummary> {
        let versions: Vec<FactVersion> =
            self.storage
                .read_json_optional(&format!("{base_dir}/{entry}/versions.json"))
                .ok()
                .flatten()?;

        let current_version = versions.last()?.version;
        let v_path = format!("{base_dir}/{entry}/v{current_version}.json");

        let json: serde_json::Value = self.storage.read_json_optional(&v_path).ok().flatten()?;

        let id = json.get("id")?.as_str()?.to_string();
        let created_at = json
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let content = json
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let title = content.chars().take(80).collect::<String>();
        let findings_count = json
            .get("findings")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        Some(FactSummary {
            id,
            fact_type,
            title,
            created_at,
            version: current_version,
            findings_count,
        })
    }

    pub fn correct(
        &self,
        fact_id: &str,
        content: &str,
        reason: &str,
        presumed_date: Option<&str>,
    ) -> AppResult<UserReportedFact> {
        if !fact_id.starts_with("urfact-") {
            return Err(AppError::DataFileNotFound(format!(
                "cannot correct non-user-reported fact: {fact_id}"
            )));
        }

        let dir = format!("facts/user-reported/{fact_id}");
        let versions: Vec<FactVersion> = self
            .storage
            .read_json(&format!("{dir}/versions.json"))?;

        let current = versions.last().ok_or_else(|| {
            AppError::DataFileNotFound(format!("no versions for fact: {fact_id}"))
        })?;

        let old_fact: UserReportedFact = self
            .storage
            .read_json(&format!("{dir}/v{}.json", current.version))?;

        let new_version = current.version + 1;
        let now = MemoryStorage::now_iso();

        let disclaimer = build_disclaimer();
        let combined = format!("{disclaimer}\n\n{content}");
        let tokens = MemoryStorage::estimate_tokens(&combined);

        if tokens > MAX_USER_REPORTED_TOKENS {
            return Err(AppError::Io(format!(
                "content exceeds token limit: {tokens}/{MAX_USER_REPORTED_TOKENS} tokens"
            )));
        }

        let corrected = UserReportedFact {
            id: old_fact.id.clone(),
            fact_type: FactType::UserReported,
            version: new_version,
            created_at: now.clone(),
            author: "agent".into(),
            presumed_date: presumed_date
                .map(|s| s.to_string())
                .or_else(|| old_fact.presumed_date.clone()),
            presumed_author: old_fact.presumed_author.clone(),
            content: content.to_string(),
            disclaimer,
            findings: old_fact.findings.clone(),
        };

        let mut new_versions = versions.clone();
        new_versions.push(FactVersion {
            version: new_version,
            created_at: now,
            reason: reason.to_string(),
        });

        self.storage.atomic_write(
            &format!("{dir}/v{new_version}.json"),
            &serde_json::to_string_pretty(&corrected)?,
        )?;
        self.storage.atomic_write(
            &format!("{dir}/versions.json"),
            &serde_json::to_string_pretty(&new_versions)?,
        )?;

        let _ = auto_commit(
            &self.storage,
            &format!("memory: correct fact {fact_id} v{new_version} — {reason}"),
            &[
                &format!("{dir}/v{new_version}.json"),
                &format!("{dir}/versions.json"),
            ],
        );

        Ok(corrected)
    }

    pub fn add_finding_to_fact(&self, fact_id: &str, finding_id: &str) -> AppResult<()> {
        if fact_id.starts_with("urfact-") {
            self.append_finding_to_user_reported(fact_id, finding_id)
        } else if fact_id.starts_with("ifact-") {
            self.append_finding_to_imported(fact_id, finding_id)
        } else {
            Err(AppError::DataFileNotFound(format!(
                "unknown fact: {fact_id}"
            )))
        }
    }

    fn append_finding_to_user_reported(&self, fact_id: &str, finding_id: &str) -> AppResult<()> {
        let dir = format!("facts/user-reported/{fact_id}");
        let versions: Vec<FactVersion> = self
            .storage
            .read_json(&format!("{dir}/versions.json"))?;
        let current_version = versions.last().map(|v| v.version).unwrap_or(1);
        let v_path = format!("{dir}/v{current_version}.json");

        let mut fact: UserReportedFact = self.storage.read_json(&v_path)?;
        if !fact.findings.contains(&finding_id.to_string()) {
            fact.findings.push(finding_id.to_string());
            self.storage
                .atomic_write(&v_path, &serde_json::to_string_pretty(&fact)?)?;
        }
        Ok(())
    }

    fn append_finding_to_imported(&self, fact_id: &str, finding_id: &str) -> AppResult<()> {
        let dir = format!("facts/imported/{fact_id}");
        let versions: Vec<FactVersion> = self
            .storage
            .read_json(&format!("{dir}/versions.json"))?;
        let current_version = versions.last().map(|v| v.version).unwrap_or(1);
        let v_path = format!("{dir}/v{current_version}.json");

        let mut fact: ImportedFact = self.storage.read_json(&v_path)?;
        if !fact.findings.contains(&finding_id.to_string()) {
            fact.findings.push(finding_id.to_string());
            self.storage
                .atomic_write(&v_path, &serde_json::to_string_pretty(&fact)?)?;
        }
        Ok(())
    }

    pub fn get_backlinked_finding_ids(&self, fact_id: &str) -> AppResult<Vec<String>> {
        if fact_id.starts_with("urfact-") {
            let dir = format!("facts/user-reported/{fact_id}");
            let versions: Vec<FactVersion> = self
                .storage
                .read_json_optional(&format!("{dir}/versions.json"))?
                .unwrap_or_default();
            let current_version = versions.last().map(|v| v.version).unwrap_or(1);
            let fact: UserReportedFact = self
                .storage
                .read_json_optional(&format!("{dir}/v{current_version}.json"))?
                .unwrap_or_else(|| {
                    panic!("fact not found: {fact_id}")
                });
            Ok(fact.findings)
        } else {
            let dir = format!("facts/imported/{fact_id}");
            let versions: Vec<FactVersion> = self
                .storage
                .read_json_optional(&format!("{dir}/versions.json"))?
                .unwrap_or_default();
            let current_version = versions.last().map(|v| v.version).unwrap_or(1);
            let fact: ImportedFact = self
                .storage
                .read_json_optional(&format!("{dir}/v{current_version}.json"))?
                .unwrap_or_else(|| {
                    panic!("fact not found: {fact_id}")
                });
            Ok(fact.findings)
        }
    }

    pub fn validate_facts_exist(&self, fact_ids: &[String]) -> AppResult<()> {
        let mut missing = Vec::new();
        for id in fact_ids {
            let exists = if id.starts_with("urfact-") {
                self.storage
                    .exists(&format!("facts/user-reported/{id}/versions.json"))
            } else if id.starts_with("ifact-") {
                self.storage
                    .exists(&format!("facts/imported/{id}/versions.json"))
            } else {
                false
            };
            if !exists {
                missing.push(id.clone());
            }
        }
        if !missing.is_empty() {
            return Err(AppError::DataFileNotFound(format!(
                "fact(s) not found: {}",
                missing.join(", ")
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::storage::MemoryStorage;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_N: AtomicU32 = AtomicU32::new(0);

    fn test_store() -> FactStore {
        let n = TEST_N.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("dietology_fact_test_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        FactStore::new(Arc::new(MemoryStorage::new(dir)))
    }

    #[test]
    fn test_create_and_read_user_reported_fact() {
        let store = test_store();
        let fact = store
            .create_user_reported("Вес 85 кг", Some("2026-01-15"))
            .unwrap();

        assert_eq!(fact.content, "Вес 85 кг");
        assert_eq!(fact.version, 1);
        assert!(fact.disclaimer.contains("Данный факт сообщён пользователем"));
        assert_eq!(fact.presumed_date, Some("2026-01-15".into()));
        assert_eq!(fact.presumed_author, "user");

        let result = store.read(&fact.id).unwrap();
        match result {
            FactReadResult::UserReported { fact: f, versions, .. } => {
                assert_eq!(f.id, fact.id);
                assert_eq!(f.content, "Вес 85 кг");
                assert_eq!(versions.len(), 1);
            }
            _ => panic!("expected UserReported"),
        }
    }

    #[test]
    fn test_create_fact_oversized_content() {
        let store = test_store();
        let long_content = "x".repeat(5000);
        let result = store.create_user_reported(&long_content, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("exceeds token limit"));
    }

    #[test]
    fn test_correct_fact() {
        let store = test_store();
        let fact = store.create_user_reported("Вес 85 кг", None).unwrap();

        let corrected = store
            .correct(&fact.id, "Вес 83 кг", "пользователь уточнил", Some("2026-03-01"))
            .unwrap();

        assert_eq!(corrected.version, 2);
        assert_eq!(corrected.content, "Вес 83 кг");
        assert_eq!(corrected.presumed_date, Some("2026-03-01".into()));

        let result = store.read(&fact.id).unwrap();
        match result {
            FactReadResult::UserReported { fact: f, versions, .. } => {
                assert_eq!(f.version, 2);
                assert_eq!(versions.len(), 2);
                assert_eq!(versions[1].reason, "пользователь уточнил");
            }
            _ => panic!("expected UserReported"),
        }
    }

    #[test]
    fn test_correct_nonexistent_fact() {
        let store = test_store();
        let result = store.correct("urfact-nonexistent", "x", "reason", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_facts() {
        let store = test_store();
        store.create_user_reported("Факт A", None).unwrap();
        store.create_user_reported("Факт B", None).unwrap();

        let list = store.list(None, 0, 10).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_list_facts_pagination() {
        let store = test_store();
        store.create_user_reported("A", None).unwrap();
        store.create_user_reported("B", None).unwrap();
        store.create_user_reported("C", None).unwrap();

        let page1 = store.list(None, 0, 2).unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = store.list(None, 2, 2).unwrap();
        assert_eq!(page2.len(), 1);
    }

    #[test]
    fn test_add_finding_to_fact() {
        let store = test_store();
        let fact = store.create_user_reported("Тест", None).unwrap();

        store
            .add_finding_to_fact(&fact.id, "finding-1")
            .unwrap();

        let result = store.read(&fact.id).unwrap();
        match result {
            FactReadResult::UserReported { fact: f, .. } => {
                assert!(f.findings.contains(&"finding-1".to_string()));
            }
            _ => panic!("expected UserReported"),
        }
    }

    #[test]
    fn test_validate_facts_exist() {
        let store = test_store();
        let fact = store.create_user_reported("Тест", None).unwrap();

        assert!(store.validate_facts_exist(&[fact.id.clone()]).is_ok());
        assert!(store.validate_facts_exist(&["urfact-nonexistent".into()]).is_err());
    }

    #[test]
    fn test_read_nonexistent_fact() {
        let store = test_store();
        let result = store.read("urfact-nonexistent");
        assert!(result.is_err());
    }
}
