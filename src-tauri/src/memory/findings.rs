use std::sync::Arc;

use crate::error::{AppError, AppResult};

use super::git::auto_commit;
use super::storage::MemoryStorage;
use super::types::{
    FactSummary, FactType, Finding, FindingReadResult, FindingStatus, FindingSummary,
};

const MAX_FINDING_TOKENS: usize = 1024;

pub struct FindingStore {
    storage: Arc<MemoryStorage>,
    fact_store: Arc<super::facts::FactStore>,
}

impl FindingStore {
    pub fn new(storage: Arc<MemoryStorage>, fact_store: Arc<super::facts::FactStore>) -> Self {
        Self {
            storage,
            fact_store,
        }
    }

    pub fn create(
        &self,
        content: &str,
        based_on: &[String],
        reason: &str,
    ) -> AppResult<Finding> {
        let tokens = MemoryStorage::estimate_tokens(content);
        if tokens > MAX_FINDING_TOKENS {
            return Err(AppError::Validation(format!(
                "finding content exceeds token limit: {tokens}/{MAX_FINDING_TOKENS} tokens"
            )));
        }

        self.fact_store.validate_facts_exist(based_on)?;

        let finding_id = super::facts::generate_finding_id();
        let now = MemoryStorage::now_iso();

        let finding = Finding {
            id: finding_id.clone(),
            created_at: now,
            author: "agent".into(),
            status: FindingStatus::Active,
            foundation_changed: false,
            based_on: based_on.to_vec(),
            content: content.to_string(),
            reason: reason.to_string(),
        };

        let dir = format!("findings/{finding_id}");
        self.storage.atomic_write(
            &format!("{dir}/finding.json"),
            &serde_json::to_string_pretty(&finding)?,
        )?;

        for fact_id in based_on {
            let _ = self.fact_store.add_finding_to_fact(fact_id, &finding_id);
        }

        let _ = auto_commit(
            &self.storage,
            &format!("memory: create finding {finding_id}"),
            &[&format!("{dir}/finding.json")],
        );

        Ok(finding)
    }

    pub fn read(&self, finding_id: &str) -> AppResult<FindingReadResult> {
        let path = format!("findings/{finding_id}/finding.json");
        let finding: Finding = self.storage.read_json(&path)?;

        let based_on_facts: Vec<FactSummary> = finding
            .based_on
            .iter()
            .filter_map(|fid| {
                self.fact_store
                    .read(fid)
                    .ok()
                    .map(|fr| match fr {
                        super::types::FactReadResult::UserReported { fact, .. } => FactSummary {
                            id: fact.id,
                            fact_type: FactType::UserReported,
                            title: fact.content.chars().take(80).collect(),
                            created_at: fact.created_at,
                            version: fact.version,
                            findings_count: fact.findings.len(),
                        },
                        super::types::FactReadResult::Imported { fact, .. } => FactSummary {
                            id: fact.id,
                            fact_type: FactType::Imported,
                            title: String::new(),
                            created_at: fact.created_at,
                            version: fact.version,
                            findings_count: fact.findings.len(),
                        },
                    })
            })
            .collect();

        Ok(FindingReadResult {
            finding,
            based_on_facts,
        })
    }

    pub fn list(&self, offset: u32, limit: u32) -> AppResult<Vec<FindingSummary>> {
        let entries = self
            .storage
            .read_dir_entries("findings")
            .unwrap_or_default();

        let mut summaries: Vec<FindingSummary> = entries
            .iter()
            .filter_map(|entry| {
                let path = format!("findings/{entry}/finding.json");
                let finding =
                    self.storage.read_json_optional::<Finding>(&path).ok().flatten()?;
                Some(FindingSummary {
                    id: finding.id,
                    title: finding.content.chars().take(80).collect(),
                    created_at: finding.created_at,
                    status: finding.status,
                    foundation_changed: finding.foundation_changed,
                    based_on_fact_ids: finding.based_on,
                })
            })
            .collect();

        summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let start = offset as usize;
        if start >= summaries.len() {
            return Ok(Vec::new());
        }
        let end = std::cmp::min(start + limit as usize, summaries.len());
        Ok(summaries[start..end].to_vec())
    }

    pub fn resolve_status(
        &self,
        finding_id: &str,
        status: FindingStatus,
        reason: &str,
    ) -> AppResult<Finding> {
        let path = format!("findings/{finding_id}/finding.json");
        let mut finding: Finding = self.storage.read_json(&path)?;

        match status {
            FindingStatus::Superseded => {
                if finding.status == FindingStatus::Superseded {
                    return Err(AppError::Validation("finding is already superseded".into()));
                }
                finding.status = FindingStatus::Superseded;
            }
            FindingStatus::Active => {
                if !finding.foundation_changed {
                    return Err(AppError::Validation(
                        "cannot reaffirm: finding foundation_changed is not set".into(),
                    ));
                }
                finding.foundation_changed = false;
            }
        }

        self.storage
            .atomic_write(&path, &serde_json::to_string_pretty(&finding)?)?;

        let _ = auto_commit(
            &self.storage,
            &format!("memory: resolve finding {finding_id} — {reason}"),
            &[&path],
        );

        Ok(finding)
    }

    pub fn mark_foundation_changed(&self, fact_id: &str) -> AppResult<Vec<String>> {
        let entries = self
            .storage
            .read_dir_entries("findings")
            .unwrap_or_default();

        let mut affected = Vec::new();
        let mut affected_paths: Vec<String> = Vec::new();
        for entry in &entries {
            let path = format!("findings/{entry}/finding.json");
            if let Ok(mut finding) = self
                .storage
                .read_json_optional::<Finding>(&path)
            {
                if let Some(f) = &mut finding {
                    if f.based_on.contains(&fact_id.to_string()) {
                        f.foundation_changed = true;
                        self.storage.atomic_write(
                            &path,
                            &serde_json::to_string_pretty(f)?,
                        )?;
                        affected.push(f.id.clone());
                        affected_paths.push(path);
                    }
                }
            }
        }

        if !affected.is_empty() {
            let path_refs: Vec<&str> = affected_paths.iter().map(|s| s.as_str()).collect();
            let _ = auto_commit(
                &self.storage,
                &format!("memory: mark foundation_changed for fact {fact_id} — {} findings affected", affected.len()),
                &path_refs,
            );
        }

        Ok(affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::facts::FactStore;
    use crate::memory::storage::MemoryStorage;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_N: AtomicU32 = AtomicU32::new(0);

    fn test_stores() -> (FindingStore, Arc<FactStore>) {
        let n = TEST_N.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("dietology_finding_test_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        let storage = Arc::new(MemoryStorage::new(dir));
        let fact_store = Arc::new(FactStore::new(storage.clone()));
        let finding_store = FindingStore::new(storage, fact_store.clone());
        (finding_store, fact_store)
    }

    #[test]
    fn test_create_and_read_finding() {
        let (store, fact_store) = test_stores();
        let fact = fact_store.create_user_reported("Тест", None).unwrap();

        let finding = store
            .create("Низкий уровень железа", &[fact.id.clone()], "анализ рациона")
            .unwrap();

        assert_eq!(finding.content, "Низкий уровень железа");
        assert_eq!(finding.based_on, vec![fact.id.clone()]);
        assert!(finding.status == FindingStatus::Active);

        let result = store.read(&finding.id).unwrap();
        assert_eq!(result.finding.id, finding.id);
        assert_eq!(result.based_on_facts.len(), 1);
    }

    #[test]
    fn test_create_finding_nonexistent_fact() {
        let (store, _) = test_stores();
        let result = store.create("Тест", &["urfact-nonexistent".into()], "reason");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_finding_oversized() {
        let (store, fact_store) = test_stores();
        let fact = fact_store.create_user_reported("X", None).unwrap();
        let long = "x".repeat(5000);
        let result = store.create(&long, &[fact.id], "reason");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_superseded() {
        let (store, fact_store) = test_stores();
        let fact = fact_store.create_user_reported("Тест", None).unwrap();
        let finding = store.create("Вывод", &[fact.id], "reason").unwrap();

        let resolved = store
            .resolve_status(&finding.id, FindingStatus::Superseded, "устарело")
            .unwrap();
        assert!(resolved.status == FindingStatus::Superseded);
    }

    #[test]
    fn test_resolve_superseded_twice_fails() {
        let (store, fact_store) = test_stores();
        let fact = fact_store.create_user_reported("Тест", None).unwrap();
        let finding = store.create("Вывод", &[fact.id], "reason").unwrap();

        store
            .resolve_status(&finding.id, FindingStatus::Superseded, "first")
            .unwrap();
        let result = store.resolve_status(&finding.id, FindingStatus::Superseded, "second");
        assert!(result.is_err());
    }

    #[test]
    fn test_reaffirmed_without_foundation_changed_fails() {
        let (store, fact_store) = test_stores();
        let fact = fact_store.create_user_reported("Тест", None).unwrap();
        let finding = store.create("Вывод", &[fact.id], "reason").unwrap();

        let result = store.resolve_status(&finding.id, FindingStatus::Active, "reaffirm");
        assert!(result.is_err());
    }

    #[test]
    fn test_mark_foundation_changed() {
        let (store, fact_store) = test_stores();
        let fact = fact_store.create_user_reported("Тест", None).unwrap();
        let fact_id = fact.id.clone();
        let finding = store.create("Вывод", &[fact_id.clone()], "reason").unwrap();

        let affected = store.mark_foundation_changed(&fact_id).unwrap();
        assert!(affected.contains(&finding.id));

        let result = store.read(&finding.id).unwrap();
        assert!(result.finding.foundation_changed);
    }

    #[test]
    fn test_list_findings() {
        let (store, fact_store) = test_stores();
        let fact = fact_store.create_user_reported("Тест", None).unwrap();
        let fact_id = fact.id;
        store.create("A", &[fact_id.clone()], "r1").unwrap();
        store.create("B", &[fact_id], "r2").unwrap();

        let list = store.list(0, 10).unwrap();
        assert_eq!(list.len(), 2);
    }
}
