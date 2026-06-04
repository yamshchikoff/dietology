use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FactType {
    Imported,
    UserReported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Active,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactVersion {
    pub version: u64,
    pub created_at: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserReportedFact {
    pub id: String,
    #[serde(rename = "type")]
    pub fact_type: FactType,
    pub version: u64,
    pub created_at: String,
    pub author: String,
    pub presumed_date: Option<String>,
    pub presumed_author: String,
    pub content: String,
    pub disclaimer: String,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedFact {
    pub id: String,
    #[serde(rename = "type")]
    pub fact_type: FactType,
    pub version: u64,
    pub created_at: String,
    pub author: String,
    pub presumed_date: Option<String>,
    pub presumed_author: Option<String>,
    pub content: serde_json::Value,
    pub source: serde_json::Value,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub created_at: String,
    pub author: String,
    pub status: FindingStatus,
    pub foundation_changed: bool,
    pub based_on: Vec<String>,
    pub content: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterDescription {
    pub version: u64,
    pub created_at: String,
    pub content: String,
    pub based_on_facts: Vec<String>,
    pub based_on_findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveMasterDescription {
    pub active_version: u64,
    pub active_file: String,
    pub foundation_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationalPreferences {
    pub updated_at: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub fact_type: FactType,
    pub title: String,
    pub created_at: String,
    pub version: u64,
    pub findings_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSummary {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub status: FindingStatus,
    pub foundation_changed: bool,
    pub based_on_fact_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "fact_type", rename_all = "snake_case")]
pub enum FactReadResult {
    UserReported {
        fact: UserReportedFact,
        versions: Vec<FactVersion>,
        findings: Vec<FindingSummary>,
    },
    Imported {
        fact: ImportedFact,
        versions: Vec<FactVersion>,
        findings: Vec<FindingSummary>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingReadResult {
    pub finding: Finding,
    pub based_on_facts: Vec<FactSummary>,
}
