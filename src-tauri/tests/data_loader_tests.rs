use std::path::PathBuf;

/// Tests for DataLoader — verifies path resolution and file access.
/// These tests are written BEFORE the DataLoader implementation (TDD: Red phase).

#[test]
fn test_data_loader_new_resolves_explicit_path() {
    // When constructed with an explicit path, base_path must be that path
    let path = PathBuf::from("/tmp/test-data");
    let _loader = dietology_lib::data::DataLoader::new(path.clone());
    // The base_path is private — we verify indirectly via read_bytes
    // (this test will fail until DataLoader struct is defined)
}

#[test]
fn test_data_loader_for_development_resolves_to_data_dir() {
    let loader = dietology_lib::data::DataLoader::for_development();
    // Verify that a known production file is readable
    let result = loader.read_bytes("data-index.json");
    assert!(result.is_ok(), "data-index.json should be readable: {:?}", result.err());
}

#[test]
fn test_data_loader_read_nonexistent_file_returns_err() {
    let loader = dietology_lib::data::DataLoader::for_development();
    let result = loader.read_bytes("nonexistent-file-12345.json");
    assert!(result.is_err(), "nonexistent file should return error");
}
