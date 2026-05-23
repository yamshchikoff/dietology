use std::collections::BTreeSet;

use crate::data::DataLoader;
use crate::models::dri::DriOverlay;
use crate::tools::registry::{ToolFn, ToolRegistry};

fn placeholder(loader: DataLoader, phase: &'static str) -> ToolFn {
    Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
        let _ = &loader;
        Ok(format!(
            r#"{{"status": "not_implemented", "message": "Phase {} task"}}"#,
            phase
        ))
    })
}

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

    // Phase 2: placeholders
    {
        let l = loader.clone();
        registry.register(
            "describe_usda_foods",
            "Return valid enum values for USDA foods dataset filters (nutrients, food_categories)",
            empty_schema.clone(),
            placeholder(l, "2"),
        );
    }
    {
        let l = loader.clone();
        registry.register(
            "describe_who_hb",
            "Return valid enum values for WHO Hb thresholds (diagnostic_groups, severity_levels)",
            empty_schema.clone(),
            placeholder(l, "2"),
        );
    }

    // Phase 3: placeholders
    {
        let l = loader.clone();
        registry.register(
            "describe_who_anaemia",
            "Return valid enum values for WHO anaemia data (countries, years, severities)",
            empty_schema.clone(),
            placeholder(l, "3"),
        );
    }
    {
        let l = loader.clone();
        registry.register(
            "describe_who_bmi",
            "Return valid enum values for WHO BMI data (countries, years, sexes, agegroups)",
            empty_schema.clone(),
            placeholder(l, "3"),
        );
    }
    {
        let l = loader.clone();
        registry.register(
            "describe_who_diabetes",
            "Return valid enum values for WHO diabetes data (countries, years, sexes, agegroups)",
            empty_schema.clone(),
            placeholder(l, "3"),
        );
    }

    // Phase 4: placeholder
    registry.register(
        "describe_lab_ranges",
        "Return valid enum values for lab reference ranges (categories, tests)",
        empty_schema.clone(),
        placeholder(loader.clone(), "4"),
    );
}
