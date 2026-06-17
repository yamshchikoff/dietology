use std::sync::Arc;

use crate::error::{AppError, AppResult};

use super::git::auto_commit;
use super::storage::MemoryStorage;
use super::types::ConversationalPreferences;

const MAX_PREFS_TOKENS: usize = 1024;

pub struct PreferencesStore {
    storage: Arc<MemoryStorage>,
}

impl PreferencesStore {
    pub fn new(storage: Arc<MemoryStorage>) -> Self {
        Self { storage }
    }

    pub fn read(&self) -> AppResult<ConversationalPreferences> {
        self.storage
            .read_json("conversational-preferences/current.json")
    }

    pub fn read_optional(&self) -> AppResult<Option<ConversationalPreferences>> {
        self.storage
            .read_json_optional("conversational-preferences/current.json")
    }

    pub fn rewrite(&self, content: &str) -> AppResult<()> {
        let tokens = MemoryStorage::estimate_tokens(content);
        if tokens > MAX_PREFS_TOKENS {
            return Err(AppError::Validation(format!(
                "preferences exceed token limit: {tokens}/{MAX_PREFS_TOKENS} tokens"
            )));
        }

        let backup_path = "conversational-preferences/backup.json";
        let current_path = "conversational-preferences/current.json";

        // Snapshot old content before writing — but write new data FIRST,
        // then update backup only on success, so a failed write doesn't destroy the backup.
        let had_current = self.storage.exists(current_path);
        let old_data = if had_current {
            Some(
                std::fs::read_to_string(self.storage.path_for(current_path)?)
                    .map_err(|e| AppError::Io(format!("read current prefs: {e}")))?,
            )
        } else {
            None
        };

        let now = MemoryStorage::now_iso();
        let prefs = ConversationalPreferences {
            updated_at: now,
            content: content.to_string(),
        };

        self.storage
            .atomic_write(current_path, &serde_json::to_string_pretty(&prefs)?)?;

        // Only overwrite backup after current write succeeds.
        if let Some(data) = old_data {
            self.storage.atomic_write(backup_path, &data)?;
        }

        let _ = auto_commit(
            &self.storage,
            "memory: rewrite conversational preferences",
            &[current_path, backup_path],
        );

        Ok(())
    }

    pub fn restore(&self) -> AppResult<()> {
        let backup_path = "conversational-preferences/backup.json";
        let current_path = "conversational-preferences/current.json";

        if !self.storage.exists(backup_path) {
            return Err(AppError::DataFileNotFound(
                "no backup to restore".into(),
            ));
        }

        let backup_data =
            std::fs::read_to_string(self.storage.path_for(backup_path)?).map_err(|e| {
                AppError::Io(format!("read backup: {e}"))
            })?;

        let current_data = if self.storage.exists(current_path) {
            Some(std::fs::read_to_string(self.storage.path_for(current_path)?).map_err(|e| {
                AppError::Io(format!("read current: {e}"))
            })?)
        } else {
            None
        };

        if let Some(data) = current_data {
            self.storage.atomic_write(backup_path, &data)?;
        }
        self.storage.atomic_write(current_path, &backup_data)?;

        let _ = auto_commit(
            &self.storage,
            "memory: restore conversational preferences from backup",
            &[current_path, backup_path],
        );

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

    fn test_store() -> PreferencesStore {
        let n = TEST_N.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("dietology_prefs_test_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        PreferencesStore::new(Arc::new(MemoryStorage::new(dir)))
    }

    #[test]
    fn test_rewrite_and_read() {
        let store = test_store();
        store.rewrite("предпочтения v1").unwrap();

        let prefs = store.read().unwrap();
        assert_eq!(prefs.content, "предпочтения v1");
    }

    #[test]
    fn test_backup_and_restore() {
        let store = test_store();
        store.rewrite("v1").unwrap();
        store.rewrite("v2").unwrap();

        assert_eq!(store.read().unwrap().content, "v2");

        store.restore().unwrap();
        assert_eq!(store.read().unwrap().content, "v1");

        store.restore().unwrap();
        assert_eq!(store.read().unwrap().content, "v2");
    }

    #[test]
    fn test_restore_no_backup_fails() {
        let store = test_store();
        let result = store.restore();
        assert!(result.is_err());
    }

    #[test]
    fn test_oversized_content() {
        let store = test_store();
        let long = "x".repeat(10000);
        let result = store.rewrite(&long);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_nonexistent() {
        let store = test_store();
        let result = store.read_optional().unwrap();
        assert!(result.is_none());
    }
}
