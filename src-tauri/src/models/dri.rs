use crate::error::AppResult;
use serde::{Deserialize, Serialize};

/// Single demographic group within a nutrient (e.g., "male_19_30yr", RDA=1000mg)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriGroup {
    pub group: String,
    pub sex: Option<String>,
    pub age_range: String,
    pub value: f64,
    #[serde(rename = "type")]
    pub dri_type: String,
    #[serde(default)]
    pub ul: Option<f64>,
    #[serde(default)]
    pub ul_unit: Option<String>,
    #[serde(default)]
    pub ul_note: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// A nutrient entry in a DRI overlay (e.g., "Calcium" with 22 groups)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriNutrient {
    pub name: String,
    pub unit: String,
    #[serde(default)]
    pub category: Option<String>,
    pub source_id: String,
    pub source_urls: Vec<String>,
    pub groups: Vec<DriGroup>,
    #[serde(default)]
    pub ul: Option<f64>,
    #[serde(default)]
    pub ul_unit: Option<String>,
    #[serde(default)]
    pub ul_note: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Any DRI overlay file (minerals, vitamins, per-kg)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriOverlay {
    pub nutrients: Vec<DriNutrient>,
}

impl DriOverlay {
    /// Read a DRI overlay file via DataLoader, extracting nutrients and ignoring _meta
    pub fn from_file(loader: &crate::data::DataLoader, path: &str) -> AppResult<Self> {
        let mut value: serde_json::Value = loader.read_json(path)?;
        let nutrients: Vec<DriNutrient> = serde_json::from_value(value["nutrients"].take())?;
        Ok(Self { nutrients })
    }
}
