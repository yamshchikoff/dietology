use std::collections::BTreeSet;

use crate::data::DataLoader;
use crate::models::dri::DriOverlay;
use crate::tools::registry::{ToolFn, ToolRegistry};

/// Extract enum values from a DRI overlay file.
fn describe_dri_overlay(loader: &DataLoader, path: &str) -> Result<String, String> {
    let overlay: DriOverlay = loader.read_json(path).map_err(|e| e.to_string())?;

    let nutrients: Vec<String> = overlay.nutrients.iter().map(|n| n.name.clone()).collect();
    let mut groups = BTreeSet::new();
    let mut sexes = BTreeSet::new();
    let mut total_groups: usize = 0;

    for n in &overlay.nutrients {
        for g in &n.groups {
            groups.insert(g.group.clone());
            if let Some(ref s) = g.sex {
                sexes.insert(s.clone());
            }
            total_groups += 1;
        }
    }

    let groups: Vec<String> = groups.into_iter().collect();
    let sexes: Vec<String> = sexes.into_iter().collect();

    let result = serde_json::json!({
        "status": "ok",
        "nutrients": nutrients,
        "groups": groups,
        "sexes": sexes,
        "total_groups": total_groups,
    });

    Ok(result.to_string())
}

/// Per-kg variant: same enum extraction plus unit and body-weight convention note.
fn describe_dri_per_kg(loader: &DataLoader, path: &str) -> Result<String, String> {
    let overlay: DriOverlay = loader.read_json(path).map_err(|e| e.to_string())?;

    let nutrients: Vec<String> = overlay.nutrients.iter().map(|n| n.name.clone()).collect();
    let mut groups = BTreeSet::new();
    let mut sexes = BTreeSet::new();
    let mut total_groups: usize = 0;

    for n in &overlay.nutrients {
        for g in &n.groups {
            groups.insert(g.group.clone());
            if let Some(ref s) = g.sex {
                sexes.insert(s.clone());
            }
            total_groups += 1;
        }
    }

    let groups: Vec<String> = groups.into_iter().collect();
    let sexes: Vec<String> = sexes.into_iter().collect();

    let unit = overlay
        .nutrients
        .first()
        .map(|n| n.unit.clone())
        .unwrap_or_else(|| "mg/kg".to_string());

    let result = serde_json::json!({
        "status": "ok",
        "nutrients": nutrients,
        "groups": groups,
        "sexes": sexes,
        "total_groups": total_groups,
        "unit": unit,
        "note": "All values in mg/kg of body weight. Multiply by individual body weight for absolute daily intake.",
    });

    Ok(result.to_string())
}

fn placeholder(loader: DataLoader, phase: &'static str) -> ToolFn {
    Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
        let _ = &loader;
        Ok(format!(
            r#"{{"status": "not_implemented", "message": "Phase {} task"}}"#,
            phase
        ))
    })
}

/// Register all 9 describe tools. Phase 1 tools have real implementations.
pub fn register_describe_tools(registry: &mut ToolRegistry, loader: &DataLoader) {
    let empty_schema = serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    });

    // Phase 1: DRI describe tools (implemented)
    {
        let ldr = loader.clone();
        registry.register(
            "describe_dri_minerals",
            "Return valid enum values for DRI minerals dataset filters (nutrients, groups, sexes)",
            empty_schema.clone(),
            Box::new(move |_args| describe_dri_overlay(&ldr, "dri-minerals-overlay.json")),
        );
    }
    {
        let ldr = loader.clone();
        registry.register(
            "describe_dri_vitamins",
            "Return valid enum values for DRI vitamins dataset filters (nutrients, groups, sexes)",
            empty_schema.clone(),
            Box::new(move |_args| describe_dri_overlay(&ldr, "dri-vitamins-overlay.json")),
        );
    }
    {
        let ldr = loader.clone();
        registry.register(
            "describe_dri_per_kg",
            "Return valid enum values for DRI per-kg dataset filters (nutrients, groups, unit)",
            empty_schema.clone(),
            Box::new(move |_args| describe_dri_per_kg(&ldr, "dri-macronutrients-per-kg-overlay.json")),
        );
    }

    // Phase 2-4: remaining placeholders
    macro_rules! reg {
        ($name:literal, $desc:literal, $phase:literal) => {
            registry.register(
                $name,
                $desc,
                empty_schema.clone(),
                placeholder(loader.clone(), $phase),
            )
        };
    }

    reg!("describe_usda_foods",
        "Return valid enum values for USDA foods dataset filters (nutrients, food_categories)",
        "2");
    reg!("describe_who_hb",
        "Return valid enum values for WHO Hb thresholds (diagnostic_groups, severity_levels)",
        "2");
    reg!("describe_who_anaemia",
        "Return valid enum values for WHO anaemia data (countries, years, severities)",
        "3");
    reg!("describe_who_bmi",
        "Return valid enum values for WHO BMI data (countries, years, sexes, agegroups)",
        "3");
    reg!("describe_who_diabetes",
        "Return valid enum values for WHO diabetes data (countries, years, sexes, agegroups)",
        "3");
    reg!("describe_lab_ranges",
        "Return valid enum values for lab reference ranges (categories, tests)",
        "4");
}
