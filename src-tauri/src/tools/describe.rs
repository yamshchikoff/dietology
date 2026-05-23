use std::collections::{BTreeMap, BTreeSet};

use crate::data::DataLoader;
use crate::models::datasets::{LabReferenceRanges, UsdaFoods, WhoEpiData, WhoHbThresholds};
use crate::models::dri::DriOverlay;
use crate::tools::registry::ToolRegistry;

fn build_dri_describe(overlay: &DriOverlay, include_sexes: bool) -> serde_json::Value {
    let nutrients: Vec<&str> = overlay.nutrients.iter().map(|n| n.name.as_str()).collect();

    let groups: BTreeSet<&str> = overlay
        .nutrients
        .iter()
        .flat_map(|n| n.groups.iter().map(|g| g.group.as_str()))
        .collect();

    let total_groups: usize = overlay.nutrients.iter().map(|n| n.groups.len()).sum();

    let mut result = serde_json::json!({
        "status": "ok",
        "nutrients": nutrients,
        "groups": groups.iter().collect::<Vec<_>>(),
        "total_groups": total_groups,
    });

    if include_sexes {
        let sexes: BTreeSet<&str> = overlay
            .nutrients
            .iter()
            .flat_map(|n| n.groups.iter().filter_map(|g| g.sex.as_deref()))
            .filter(|s| *s != "any")
            .collect();
        result["sexes"] = serde_json::json!(sexes.iter().collect::<Vec<_>>());
    }

    result
}

fn describe_dri_impl(loader: &DataLoader, path: &str, include_sexes: bool) -> Result<String, String> {
    let overlay: DriOverlay = loader
        .read_json(path)
        .map_err(|e| format!("failed to read {path}: {e}"))?;
    Ok(build_dri_describe(&overlay, include_sexes).to_string())
}

fn build_usda_foods_describe(foods: &UsdaFoods) -> serde_json::Value {
    let nutrients: BTreeSet<&str> = foods
        .foods
        .iter()
        .flat_map(|f| f.nutrients.keys().map(|k| k.as_str()))
        .collect();

    let food_categories: BTreeSet<&str> =
        foods.foods.iter().map(|f| f.category.as_str()).collect();

    serde_json::json!({
        "status": "ok",
        "nutrients": nutrients.iter().collect::<Vec<_>>(),
        "food_categories": food_categories.iter().collect::<Vec<_>>(),
        "total_foods": foods.foods.len(),
    })
}

fn build_who_hb_describe(hb: &WhoHbThresholds) -> serde_json::Value {
    let diagnostic_groups: Vec<&str> = hb
        .diagnostic_thresholds
        .iter()
        .map(|t| t.group.as_str())
        .collect();

    let sexes: BTreeSet<&str> = hb
        .diagnostic_thresholds
        .iter()
        .map(|t| t.sex.as_str())
        .collect();

    let pregnant_options: BTreeSet<bool> = hb
        .diagnostic_thresholds
        .iter()
        .map(|t| t.pregnant)
        .collect();

    serde_json::json!({
        "status": "ok",
        "diagnostic_groups": diagnostic_groups,
        "severity_levels": ["normal", "mild", "moderate", "severe"],
        "sexes": sexes.iter().collect::<Vec<_>>(),
        "pregnant_options": pregnant_options.iter().collect::<Vec<_>>(),
        "total_thresholds": hb.diagnostic_thresholds.len(),
    })
}

fn build_epi_describe(data: &WhoEpiData) -> serde_json::Value {
    let countries: BTreeSet<&str> = data
        .data
        .iter()
        .map(|r| r.country_code.as_str())
        .collect();

    let years_min = data.data.iter().map(|r| r.year).min().unwrap_or(0);
    let years_max = data.data.iter().map(|r| r.year).max().unwrap_or(0);

    let sexes: BTreeSet<&str> = data
        .data
        .iter()
        .filter_map(|r| r.sex.as_deref())
        .collect();

    let agegroups: BTreeSet<&str> = data
        .data
        .iter()
        .filter_map(|r| r.agegroup.as_deref())
        .collect();

    let severities: BTreeSet<&str> = data
        .data
        .iter()
        .filter_map(|r| r.severity.as_deref())
        .collect();

    let mut result = serde_json::json!({
        "status": "ok",
        "countries": countries.iter().collect::<Vec<_>>(),
        "years": {"min": years_min, "max": years_max},
        "total_records": data.data.len(),
    });

    if !sexes.is_empty() {
        result["sexes"] = serde_json::json!(sexes.iter().collect::<Vec<_>>());
    }
    if !agegroups.is_empty() {
        result["agegroups"] = serde_json::json!(agegroups.iter().collect::<Vec<_>>());
    }
    if !severities.is_empty() {
        result["severities"] = serde_json::json!(severities.iter().collect::<Vec<_>>());
    }

    result
}

fn build_lab_ranges_describe(lr: &LabReferenceRanges) -> serde_json::Value {
    let mut category_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for r in &lr.ranges {
        *category_counts.entry(r.category.as_str()).or_insert(0) += 1;
    }

    serde_json::json!({
        "status": "ok",
        "categories": category_counts.iter().map(|(name, count)| {
            serde_json::json!({"name": name, "count": count})
        }).collect::<Vec<_>>(),
        "total_tests": lr.ranges.len(),
    })
}

/// Register all 9 describe tools. Each handler captures a DataLoader clone.
pub fn register_describe_tools(registry: &mut ToolRegistry, loader: &DataLoader) {
    let empty_schema = serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    });

    // Phase 1: DRI describe tools (real implementations)
    {
        let l = loader.clone();
        registry.register(
            "describe_dri_minerals",
            "Return valid enum values for DRI minerals dataset filters (nutrients, groups, sexes)",
            empty_schema.clone(),
            Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
                describe_dri_impl(&l, "dri-minerals-overlay.json", true)
            }),
        );
    }
    {
        let l = loader.clone();
        registry.register(
            "describe_dri_vitamins",
            "Return valid enum values for DRI vitamins dataset filters (nutrients, groups, sexes)",
            empty_schema.clone(),
            Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
                describe_dri_impl(&l, "dri-vitamins-overlay.json", true)
            }),
        );
    }
    {
        let l = loader.clone();
        registry.register(
            "describe_dri_per_kg",
            "Return valid enum values for DRI per-kg dataset filters (nutrients, groups, unit)",
            empty_schema.clone(),
            Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
                let raw: serde_json::Value = l
                    .read_json("dri-macronutrients-per-kg-overlay.json")
                    .map_err(|e| format!("failed to read per-kg overlay: {e}"))?;
                let overlay: DriOverlay = serde_json::from_value(raw.clone())
                    .map_err(|e| format!("failed to deserialize per-kg overlay: {e}"))?;
                let mut result = build_dri_describe(&overlay, false);
                result["unit"] = serde_json::json!("mg/kg");
                result["note"] = serde_json::json!(
                    raw["_meta"]["note"]
                        .as_str()
                        .ok_or("missing _meta.note in per-kg overlay")?
                );
                Ok(result.to_string())
            }),
        );
    }

    // Phase 2: USDA Foods + WHO Hb thresholds (real implementations)
    {
        let l = loader.clone();
        registry.register(
            "describe_usda_foods",
            "Return valid enum values for USDA foods dataset filters (nutrients, food_categories)",
            empty_schema.clone(),
            Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
                let foods: UsdaFoods = l
                    .read_json("usda-foundation-foods-essential.json")
                    .map_err(|e| format!("failed to read USDA foods: {e}"))?;
                Ok(build_usda_foods_describe(&foods).to_string())
            }),
        );
    }
    {
        let l = loader.clone();
        registry.register(
            "describe_who_hb",
            "Return valid enum values for WHO Hb thresholds (diagnostic_groups, severity_levels, sexes, pregnant_options)",
            empty_schema.clone(),
            Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
                let hb: WhoHbThresholds = l
                    .read_json("who-hb-thresholds.json")
                    .map_err(|e| format!("failed to read WHO Hb thresholds: {e}"))?;
                Ok(build_who_hb_describe(&hb).to_string())
            }),
        );
    }

    // Phase 3: WHO GHO epidemiology (real implementations)
    {
        let l = loader.clone();
        registry.register(
            "describe_who_anaemia",
            "Return valid enum values for WHO anaemia data (countries, years, sexes, severities)",
            empty_schema.clone(),
            Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
                let data: WhoEpiData = l
                    .read_json("who-anaemia-nonpregnant-prevalence.json")
                    .map_err(|e| format!("failed to read WHO anaemia data: {e}"))?;
                Ok(build_epi_describe(&data).to_string())
            }),
        );
    }
    {
        let l = loader.clone();
        registry.register(
            "describe_who_bmi",
            "Return valid enum values for WHO BMI data (countries, years, sexes, agegroups)",
            empty_schema.clone(),
            Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
                let data: WhoEpiData = l
                    .read_json("who-bmi-overweight-prevalence.json")
                    .map_err(|e| format!("failed to read WHO BMI data: {e}"))?;
                Ok(build_epi_describe(&data).to_string())
            }),
        );
    }
    {
        let l = loader.clone();
        registry.register(
            "describe_who_diabetes",
            "Return valid enum values for WHO diabetes data (countries, years, sexes, agegroups)",
            empty_schema.clone(),
            Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
                let data: WhoEpiData = l
                    .read_json("who-diabetes-prevalence.json")
                    .map_err(|e| format!("failed to read WHO diabetes data: {e}"))?;
                Ok(build_epi_describe(&data).to_string())
            }),
        );
    }

    // Phase 4: Lab reference ranges (real implementation)
    {
        let l = loader.clone();
        registry.register(
            "describe_lab_ranges",
            "Return valid enum values for lab reference ranges (categories with test counts, total_tests)",
            empty_schema.clone(),
            Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
                let lr: LabReferenceRanges = l
                    .read_json("lab-reference-ranges.json")
                    .map_err(|e| format!("failed to read lab reference ranges: {e}"))?;
                Ok(build_lab_ranges_describe(&lr).to_string())
            }),
        );
    }
}
