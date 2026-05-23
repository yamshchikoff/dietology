use crate::error::AppResult;
use std::path::PathBuf;

/// Registry of all 11 production JSON files (logical name → relative path from data/).
pub const PRODUCTION_FILES: &[(&str, &str)] = &[
    ("sources_final", "sources-final.json"),
    ("data_index", "data-index.json"),
    ("dri_minerals", "dri-minerals-overlay.json"),
    ("dri_vitamins", "dri-vitamins-overlay.json"),
    ("dri_per_kg", "dri-macronutrients-per-kg-overlay.json"),
    ("usda_foods", "usda-foundation-foods-essential.json"),
    ("who_hb", "who-hb-thresholds.json"),
    ("who_anaemia", "who-anaemia-nonpregnant-prevalence.json"),
    ("who_bmi", "who-bmi-overweight-prevalence.json"),
    ("who_diabetes", "who-diabetes-prevalence.json"),
    ("lab_ranges", "lab-reference-ranges.json"),
];

pub struct DataLoader {
    base_path: PathBuf,
}

impl DataLoader {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn for_development() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        Self::new(manifest_dir.join("..").join("data"))
    }

    pub fn read_bytes(&self, relative_path: &str) -> AppResult<Vec<u8>> {
        let full_path = self.base_path.join(relative_path);
        Ok(std::fs::read(&full_path)?)
    }

    pub fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        relative_path: &str,
    ) -> AppResult<T> {
        let bytes = self.read_bytes(relative_path)?;
        let value = serde_json::from_slice(&bytes)?;
        Ok(value)
    }
}

/// Verify all 11 production files are readable. Returns the list of missing file names.
pub fn verify_all_production_files(loader: &DataLoader) -> Vec<String> {
    let mut missing = Vec::new();
    for (_name, path) in PRODUCTION_FILES {
        if loader.read_bytes(path).is_err() {
            missing.push(path.to_string());
        }
    }
    missing
}
