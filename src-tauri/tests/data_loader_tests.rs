use std::path::PathBuf;

#[test]
fn test_data_loader_new_resolves_explicit_path() {
    let path = PathBuf::from("/tmp/test-data");
    let loader = dietology_lib::data::DataLoader::new(path.clone());
    let result = loader.read_bytes("any-file.json");
    assert!(result.is_err());
}

#[test]
fn test_data_loader_read_json_deserializes_manifest() {
    let loader = dietology_lib::data::DataLoader::for_development();
    let index: dietology_lib::models::manifest::DataIndex =
        loader.read_json("data-index.json").unwrap();
    assert!(!index.datasets.is_empty(), "data-index should have datasets");
    assert!(index.stats.total_dri_nutrients > 0);
}

#[test]
fn test_data_loader_for_development_resolves_to_data_dir() {
    let loader = dietology_lib::data::DataLoader::for_development();
    let result = loader.read_bytes("data-index.json");
    assert!(result.is_ok(), "data-index.json should be readable: {:?}", result.err());
}

#[test]
fn test_data_loader_read_nonexistent_file_returns_err() {
    let loader = dietology_lib::data::DataLoader::for_development();
    let result = loader.read_bytes("nonexistent-file-12345.json");
    assert!(result.is_err(), "nonexistent file should return error");
}

#[test]
fn test_all_production_files_accessible() {
    let loader = dietology_lib::data::DataLoader::for_development();
    let missing = dietology_lib::data::verify_all_production_files(&loader);
    assert!(
        missing.is_ok(),
        "Production files missing: {:?}",
        missing.err()
    );
    assert!(
        missing.unwrap().is_empty(),
        "Some production files are missing"
    );
}

#[test]
fn test_production_files_count() {
    let count = dietology_lib::data::PRODUCTION_FILES.len();
    assert_eq!(count, 11, "Expected exactly 11 production files, got {count}");
}
