use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::error::{AppError, AppResult};

pub struct MemoryStorage {
    base_path: PathBuf,
}

impl MemoryStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn for_development() -> Self {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
        Self::new(PathBuf::from(&manifest_dir).join("..").join("data"))
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    fn validate_path(relative_path: &str) -> AppResult<()> {
        if relative_path.is_empty() {
            return Err(AppError::Validation("path is empty".into()));
        }
        if relative_path.contains('\0') {
            return Err(AppError::Validation("path contains null byte".into()));
        }
        let p = Path::new(relative_path);
        if p.is_absolute() {
            return Err(AppError::Validation("absolute paths not allowed".into()));
        }
        if p.components().any(|c| c == std::path::Component::ParentDir) {
            return Err(AppError::Validation("'..' not allowed in path".into()));
        }
        Ok(())
    }

    pub fn read_json<T: DeserializeOwned>(&self, relative_path: &str) -> AppResult<T> {
        Self::validate_path(relative_path)?;
        let full_path = self.base_path.join(relative_path);
        let data = fs::read_to_string(&full_path).map_err(|e| {
            AppError::DataFileNotFound(format!("{}: {e}", full_path.display()))
        })?;
        serde_json::from_str(&data).map_err(AppError::from)
    }

    pub fn read_json_optional<T: DeserializeOwned>(
        &self,
        relative_path: &str,
    ) -> AppResult<Option<T>> {
        let full_path = self.base_path.join(relative_path);
        if !full_path.exists() {
            return Ok(None);
        }
        self.read_json(relative_path).map(Some)
    }

    pub fn read_dir_entries(&self, relative_path: &str) -> AppResult<Vec<String>> {
        Self::validate_path(relative_path)?;
        let full_path = self.base_path.join(relative_path);
        if !full_path.exists() {
            return Ok(Vec::new());
        }
        let mut entries: Vec<String> = fs::read_dir(&full_path)
            .map_err(|e| AppError::Io(format!("read_dir {}: {e}", full_path.display())))?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if name.starts_with('.') {
                        None
                    } else {
                        Some(name)
                    }
                })
            })
            .collect();
        entries.sort();
        Ok(entries)
    }

    pub fn atomic_write(&self, relative_path: &str, data: &str) -> AppResult<()> {
        Self::validate_path(relative_path)?;
        let full_path = self.base_path.join(relative_path);
        let tmp_path = full_path.with_extension("tmp");

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AppError::Io(format!("create_dir {}: {e}", parent.display()))
            })?;
        }

        let mut file = fs::File::create(&tmp_path)
            .map_err(|e| AppError::Io(format!("create {}: {e}", tmp_path.display())))?;
        file.write_all(data.as_bytes())
            .map_err(|e| AppError::Io(format!("write {}: {e}", tmp_path.display())))?;
        file.sync_all()
            .map_err(|e| AppError::Io(format!("fsync {}: {e}", tmp_path.display())))?;

        fs::rename(&tmp_path, &full_path)
            .map_err(|e| AppError::Io(format!("rename {} -> {}: {e}", tmp_path.display(), full_path.display())))?;

        Ok(())
    }

    pub fn estimate_tokens(text: &str) -> usize {
        tiktoken::get_encoding("cl100k_base")
            .expect("cl100k_base encoding not found")
            .count(text)
    }

    pub fn path_for(&self, relative: &str) -> AppResult<PathBuf> {
        Self::validate_path(relative)?;
        Ok(self.base_path.join(relative))
    }

    pub fn exists(&self, relative_path: &str) -> bool {
        Self::validate_path(relative_path).is_ok() && self.base_path.join(relative_path).exists()
    }

    pub fn now_iso() -> String {
        let dur = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = dur.as_secs();
        let days = secs / 86400;
        let time_secs = secs % 86400;
        let hours = time_secs / 3600;
        let mins = (time_secs % 3600) / 60;
        let secs_remainder = time_secs % 60;

        let (year, month, day) = civil_from_days(days as i64 + 719468);
        format!("{year:04}-{month:02}-{day:02}T{hours:02}:{mins:02}:{secs_remainder:02}Z")
    }
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_storage() -> MemoryStorage {
        let dir = std::env::temp_dir().join(format!("dietology_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        MemoryStorage::new(dir)
    }

    #[test]
    fn test_atomic_write_and_read() {
        let storage = temp_storage();
        let path = "test_atomic/subdir/data.json";
        storage.atomic_write(path, "{\"key\":42}").unwrap();

        let content: serde_json::Value = storage.read_json(path).unwrap();
        assert_eq!(content["key"], 42);
    }

    #[test]
    fn test_atomic_write_overwrite() {
        let storage = temp_storage();
        let path = "test_overwrite.json";
        storage.atomic_write(path, "v1").unwrap();
        storage.atomic_write(path, "v2").unwrap();

        let result = std::fs::read_to_string(storage.path_for(path).unwrap()).unwrap();
        assert_eq!(result, "v2");
    }

    #[test]
    fn test_read_nonexistent() {
        let storage = temp_storage();
        let result: AppResult<serde_json::Value> = storage.read_json("nonexistent.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_json_optional() {
        let storage = temp_storage();
        let result: Option<serde_json::Value> = storage.read_json_optional("nonexistent.json").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_read_dir_entries() {
        let storage = temp_storage();
        storage.atomic_write("entries/a.json", "{}").unwrap();
        storage.atomic_write("entries/b.json", "{}").unwrap();

        let entries = storage.read_dir_entries("entries").unwrap();
        assert_eq!(entries, vec!["a.json", "b.json"]);
    }

    #[test]
    fn test_read_dir_empty() {
        let storage = temp_storage();
        let entries = storage.read_dir_entries("nonexistent_dir").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_estimate_tokens() {
        // cl100k_base tokenizer
        assert_eq!(MemoryStorage::estimate_tokens("hello"), 1);
        assert_eq!(MemoryStorage::estimate_tokens(""), 0);
        assert_eq!(MemoryStorage::estimate_tokens("Привет, мир!"), 7);
    }

    #[test]
    fn test_exists() {
        let storage = temp_storage();
        assert!(!storage.exists("test_exists.json"));
        storage.atomic_write("test_exists.json", "x").unwrap();
        assert!(storage.exists("test_exists.json"));
    }
}
