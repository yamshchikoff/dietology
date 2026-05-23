use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============ USDA Foundation Foods ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrientAmount {
    pub amount: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Food {
    pub name: String,
    pub category: String,
    #[serde(rename = "fdcId")]
    pub fdc_id: u64,
    pub nutrients: HashMap<String, NutrientAmount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdaFoods {
    pub foods: Vec<Food>,
}

// ============ WHO Hb Thresholds ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HbDiagnosticThreshold {
    pub group: String,
    pub sex: String,
    pub pregnant: bool,
    pub hb_cutoff_g_per_l: f64,
    pub hb_cutoff_g_per_dl: f64,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HbSeverityRange {
    pub group: String,
    pub normal_low: f64,
    pub mild_low: f64,
    pub mild_high: f64,
    pub moderate_low: f64,
    pub moderate_high: f64,
    pub severe_below: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoHbThresholds {
    pub diagnostic_thresholds: Vec<HbDiagnosticThreshold>,
    pub severity_classification: Vec<HbSeverityRange>,
}

// ============ WHO GHO Epidemiology ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpiRecord {
    pub country_code: String,
    pub year: u32,
    pub value: f64,
    pub low: f64,
    pub high: f64,
    #[serde(default)]
    pub parent_region: Option<String>,
    #[serde(default)]
    pub parent_region_code: Option<String>,
    pub sex: String,
    #[serde(default)]
    pub agegroup: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoEpiData {
    pub data: Vec<EpiRecord>,
}

// ============ Lab Reference Ranges ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabRange {
    pub category: String,
    pub test: String,
    #[serde(default)]
    #[serde(rename = "type")]
    pub range_type: Option<String>,
    #[serde(default)]
    pub low: Option<String>,
    #[serde(default)]
    pub high: Option<String>,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabReferenceRanges {
    pub ranges: Vec<LabRange>,
}
