use std::sync::Arc;

use crate::error::{AppError, AppResult};

use super::git::auto_commit;
use super::storage::MemoryStorage;
use super::types::{ActiveMasterDescription, MasterDescription};

const MAX_MASTER_TOKENS: usize = 50_000;

pub struct LlmCredentials {
    pub api_key: String,
    pub api_base_url: String,
    pub model: String,
}

pub struct MasterDescriptionStore {
    storage: Arc<MemoryStorage>,
}

impl MasterDescriptionStore {
    pub fn new(storage: Arc<MemoryStorage>) -> Self {
        Self { storage }
    }

    pub fn read(&self) -> AppResult<(MasterDescription, bool)> {
        let active: ActiveMasterDescription =
            self.storage
                .read_json("master-description/active.json")?;
        let doc: MasterDescription = self
            .storage
            .read_json(&format!("master-description/{}", active.active_file))?;
        Ok((doc, active.foundation_changed))
    }

    pub fn read_optional(&self) -> AppResult<Option<(MasterDescription, bool)>> {
        if !self.exists() {
            return Ok(None);
        }
        self.read().map(Some)
    }

    pub fn rewrite(
        &self,
        content: &str,
        based_on_facts: Vec<String>,
        based_on_findings: Vec<String>,
    ) -> AppResult<u64> {
        let tokens = MemoryStorage::estimate_tokens(content);
        if tokens > MAX_MASTER_TOKENS {
            return Err(AppError::Validation(format!(
                "master description exceeds token limit: {tokens}/{MAX_MASTER_TOKENS} tokens"
            )));
        }

        let new_version = if self.exists() {
            let active: ActiveMasterDescription = self
                .storage
                .read_json("master-description/active.json")?;
            active.active_version + 1
        } else {
            1
        };

        let now = MemoryStorage::now_iso();
        let doc = MasterDescription {
            version: new_version,
            created_at: now,
            content: content.to_string(),
            based_on_facts,
            based_on_findings,
        };

        let file_name = format!("v{new_version}.json");
        self.storage.atomic_write(
            &format!("master-description/{file_name}"),
            &serde_json::to_string_pretty(&doc)?,
        )?;

        let active = ActiveMasterDescription {
            active_version: new_version,
            active_file: file_name.clone(),
            foundation_changed: false,
        };
        self.storage.atomic_write(
            "master-description/active.json",
            &serde_json::to_string_pretty(&active)?,
        )?;

        let _ = auto_commit(
            &self.storage,
            &format!("memory: rewrite master description v{new_version}"),
            &[
                &format!("master-description/{file_name}"),
                "master-description/active.json",
            ],
        );

        Ok(new_version)
    }

    pub fn mark_foundation_changed(&self) -> AppResult<()> {
        if !self.exists() {
            return Ok(());
        }
        let mut active: ActiveMasterDescription =
            self.storage
                .read_json("master-description/active.json")?;
        active.foundation_changed = true;
        self.storage.atomic_write(
            "master-description/active.json",
            &serde_json::to_string_pretty(&active)?,
        )?;
        let _ = auto_commit(
            &self.storage,
            "memory: mark master description foundation_changed",
            &["master-description/active.json"],
        );
        Ok(())
    }

    pub fn exists(&self) -> bool {
        self.storage
            .exists("master-description/active.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::storage::MemoryStorage;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_N: AtomicU32 = AtomicU32::new(0);

    fn test_store() -> MasterDescriptionStore {
        let n = TEST_N.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("dietology_master_test_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        MasterDescriptionStore::new(Arc::new(MemoryStorage::new(dir)))
    }

    #[test]
    fn test_rewrite_and_read() {
        let store = test_store();
        let v = store
            .rewrite(
                "Целостная картина пользователя",
                vec!["urfact-1".into()],
                vec!["finding-1".into()],
            )
            .unwrap();
        assert_eq!(v, 1);

        let (doc, fc) = store.read().unwrap();
        assert_eq!(doc.content, "Целостная картина пользователя");
        assert_eq!(doc.based_on_facts, vec!["urfact-1"]);
        assert!(!fc);
    }

    #[test]
    fn test_version_increment() {
        let store = test_store();
        store.rewrite("v1", vec![], vec![]).unwrap();
        store.rewrite("v2", vec![], vec![]).unwrap();

        let (doc, _) = store.read().unwrap();
        assert_eq!(doc.version, 2);
        assert_eq!(doc.content, "v2");
    }

    #[test]
    fn test_mark_foundation_changed() {
        let store = test_store();
        store.rewrite("test", vec![], vec![]).unwrap();

        store.mark_foundation_changed().unwrap();
        let (_, fc) = store.read().unwrap();
        assert!(fc);
    }

    #[test]
    fn test_read_nonexistent() {
        let store = test_store();
        let result = store.read_optional().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_oversized_content() {
        let store = test_store();
        let long = "x".repeat(250_000);
        let result = store.rewrite(&long, vec![], vec![]);
        assert!(result.is_err());
    }
}
