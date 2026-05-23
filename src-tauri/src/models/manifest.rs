use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============ Data Index ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetEntry {
    pub domain: String,
    pub tier: String,
    pub description: String,
    pub sources: Vec<String>,
    pub file: String,
    pub count: u64,
    pub detail: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub build_script: Option<String>,
    #[serde(default)]
    pub extraction_script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStats {
    pub total_dri_nutrients: u64,
    pub total_dri_groups: u64,
    pub total_foods: u64,
    pub total_lab_tests: u64,
    pub total_diagnostic_thresholds: u64,
    pub total_epi_records: u64,
    pub fabrication: u64,
    pub recalculation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataIndex {
    pub datasets: HashMap<String, DatasetEntry>,
    pub stats: DatasetStats,
}

// ============ Sources Final ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesFinal {
    pub schema_version: String,
    pub description: String,
    pub sources: serde_json::Value,
    pub stats: serde_json::Value,
}
