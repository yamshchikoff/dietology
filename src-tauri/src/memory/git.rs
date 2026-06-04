use std::process::Command;

use crate::error::{AppError, AppResult};

use super::storage::MemoryStorage;

pub fn auto_commit(storage: &MemoryStorage, message: &str, paths: &[&str]) -> AppResult<()> {
    let base = storage.base_path();

    if paths.is_empty() {
        return Ok(());
    }

    let add_status = Command::new("git")
        .arg("-C")
        .arg(base)
        .arg("add")
        .args(paths)
        .status()
        .map_err(|e| AppError::Io(format!("git add failed: {e}")))?;

    if !add_status.success() {
        return Err(AppError::Io(format!(
            "git add exited with {}",
            add_status.code().unwrap_or(-1)
        )));
    }

    let commit_status = Command::new("git")
        .arg("-C")
        .arg(base)
        .arg("commit")
        .arg("-m")
        .arg(message)
        .status()
        .map_err(|e| AppError::Io(format!("git commit failed: {e}")))?;

    if !commit_status.success() {
        let code = commit_status.code().unwrap_or(-1);
        if code == 1 {
            return Ok(());
        }
        return Err(AppError::Io(format!("git commit exited with {code}")));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::storage::MemoryStorage;
    use std::fs;
    use std::process::Command;

    fn setup_git_repo(path: &std::path::Path) {
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("init")
            .status()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("config")
            .arg("user.email")
            .arg("test@test.test")
            .status()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("config")
            .arg("user.name")
            .arg("test")
            .status()
            .unwrap();
    }

    #[test]
    fn test_auto_commit_creates_commit() {
        let dir = std::env::temp_dir().join(format!("dietology_git_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        setup_git_repo(&dir);

        let storage = MemoryStorage::new(dir.clone());
        storage.atomic_write("test_commit.json", "{}").unwrap();

        let result = auto_commit(&storage, "test: add test_commit.json", &["test_commit.json"]);
        assert!(result.is_ok(), "auto_commit failed: {result:?}");

        let log = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .arg("log")
            .arg("--oneline")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&log.stdout);
        assert!(
            stdout.contains("test: add test_commit.json"),
            "commit not found in log: {stdout}"
        );
    }

    #[test]
    fn test_auto_commit_empty_paths() {
        let storage = MemoryStorage::new(std::env::temp_dir().join("nonexistent"));
        let result = auto_commit(&storage, "nothing", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_auto_commit_no_changes_is_ok() {
        let dir =
            std::env::temp_dir().join(format!("dietology_git_test2_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        setup_git_repo(&dir);

        let storage = MemoryStorage::new(dir);
        storage.atomic_write("nochange.json", "x").unwrap();
        auto_commit(&storage, "initial", &["nochange.json"]).unwrap();

        let result = auto_commit(&storage, "same paths, no changes", &["nochange.json"]);
        assert!(result.is_ok());
    }
}
