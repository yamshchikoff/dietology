use crate::data::DataLoader;
use crate::models::datasets::{Food, HbDiagnosticThreshold, HbSeverityRange, UsdaFoods, WhoHbThresholds};
use crate::models::dri::{DriNutrient, DriOverlay};
use crate::tools::registry::ToolRegistry;

// ---- helpers ----

fn build_response(data: &[serde_json::Value], total_count: usize, filters: serde_json::Value) -> String {
    let result = serde_json::json!({
        "status": "ok",
        "data": data,
        "total_count": total_count,
        "filters_applied": filters,
    });
    result.to_string()
}

fn get_str_arg(args: &serde_json::Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(String::from)
}

fn get_bool_arg(args: &serde_json::Value, key: &str) -> Option<bool> {
    args.get(key).and_then(|v| v.as_bool())
}

fn get_u64_arg(args: &serde_json::Value, key: &str) -> Option<u64> {
    args.get(key).and_then(|v| v.as_u64())
}

/// Filter a DRI overlay by nutrient name and optional group/sex/pregnant/breastfeeding.
fn filter_dri_overlay(
    overlay: &DriOverlay,
    nutrient: &str,
    group: Option<&str>,
    sex: Option<&str>,
    pregnant: Option<bool>,
    breastfeeding: Option<bool>,
) -> Vec<serde_json::Value> {
    let nutrient_entry: Option<&DriNutrient> = overlay
        .nutrients
        .iter()
        .find(|n| n.name == nutrient);

    let Some(nutrient_entry) = nutrient_entry else {
        return Vec::new();
    };

    let unit = &nutrient_entry.unit;

    nutrient_entry
        .groups
        .iter()
        .filter(|g| {
            if let Some(ref grp) = group {
                if &g.group != grp {
                    return false;
                }
            }
            if let Some(s) = sex {
                if g.sex.as_deref() != Some(s) {
                    return false;
                }
            }
            if let Some(p) = pregnant {
                let is_pregnant = g.group.contains("pregnant");
                if is_pregnant != p {
                    return false;
                }
            }
            if let Some(bf) = breastfeeding {
                let is_breastfeeding = g.group.contains("breastfeeding");
                if is_breastfeeding != bf {
                    return false;
                }
            }
            true
        })
        .map(|g| {
            let mut entry = serde_json::json!({
                "group": g.group,
                "sex": g.sex,
                "age_range": g.age_range,
                "value": g.value,
                "type": g.dri_type,
                "unit": unit,
            });
            if let Some(ul) = g.ul.or(nutrient_entry.ul) {
                entry["ul"] = serde_json::json!(ul);
            }
            if let Some(ref ul_unit) = g.ul_unit.as_ref().or(nutrient_entry.ul_unit.as_ref()) {
                entry["ul_unit"] = serde_json::json!(ul_unit);
            }
            if let Some(ref ul_note) = g.ul_note.as_ref().or(nutrient_entry.ul_note.as_ref()) {
                entry["ul_note"] = serde_json::json!(ul_note);
            }
            if let Some(ref note) = g.note.as_ref().or(nutrient_entry.note.as_ref()) {
                entry["note"] = serde_json::json!(note);
            }
            entry
        })
        .collect()
}

// ---- query handlers ----

fn query_dri_minerals_impl(loader: &DataLoader, args: &serde_json::Value) -> Result<String, String> {
    let overlay: DriOverlay = loader
        .read_json("dri-minerals-overlay.json")
        .map_err(|e| format!("failed to read minerals overlay: {e}"))?;

    let nutrient = get_str_arg(args, "nutrient")
        .ok_or_else(|| "missing required parameter: nutrient".to_string())?;

    let group = get_str_arg(args, "group");
    let sex = get_str_arg(args, "sex");
    let pregnant = get_bool_arg(args, "pregnant");
    let breastfeeding = get_bool_arg(args, "breastfeeding");

    let data = filter_dri_overlay(
        &overlay,
        &nutrient,
        group.as_deref(),
        sex.as_deref(),
        pregnant,
        breastfeeding,
    );

    let mut filters = serde_json::json!({ "nutrient": nutrient });
    if let Some(g) = group {
        filters["group"] = serde_json::json!(g);
    }
    if let Some(s) = sex {
        filters["sex"] = serde_json::json!(s);
    }
    if let Some(p) = pregnant {
        filters["pregnant"] = serde_json::json!(p);
    }
    if let Some(bf) = breastfeeding {
        filters["breastfeeding"] = serde_json::json!(bf);
    }

    Ok(build_response(&data, data.len(), filters))
}

fn query_dri_vitamins_impl(loader: &DataLoader, args: &serde_json::Value) -> Result<String, String> {
    let overlay: DriOverlay = loader
        .read_json("dri-vitamins-overlay.json")
        .map_err(|e| format!("failed to read vitamins overlay: {e}"))?;

    let nutrient = get_str_arg(args, "nutrient")
        .ok_or_else(|| "missing required parameter: nutrient".to_string())?;

    let group = get_str_arg(args, "group");
    let sex = get_str_arg(args, "sex");

    let data = filter_dri_overlay(&overlay, &nutrient, group.as_deref(), sex.as_deref(), None, None);

    let mut filters = serde_json::json!({ "nutrient": nutrient });
    if let Some(g) = group {
        filters["group"] = serde_json::json!(g);
    }
    if let Some(s) = sex {
        filters["sex"] = serde_json::json!(s);
    }

    Ok(build_response(&data, data.len(), filters))
}

fn query_dri_per_kg_impl(loader: &DataLoader, args: &serde_json::Value) -> Result<String, String> {
    let overlay: DriOverlay = loader
        .read_json("dri-macronutrients-per-kg-overlay.json")
        .map_err(|e| format!("failed to read per-kg overlay: {e}"))?;

    let nutrient = get_str_arg(args, "nutrient")
        .ok_or_else(|| "missing required parameter: nutrient".to_string())?;

    let group = get_str_arg(args, "group");

    let data = filter_dri_overlay(&overlay, &nutrient, group.as_deref(), None, None, None);

    let mut filters = serde_json::json!({ "nutrient": nutrient });
    if let Some(g) = group {
        filters["group"] = serde_json::json!(g);
    }

    Ok(build_response(&data, data.len(), filters))
}

// ---- Phase 2: USDA Foods + WHO Hb ----

fn query_usda_foods_impl(loader: &DataLoader, args: &serde_json::Value) -> Result<String, String> {
    let foods_data: UsdaFoods = loader
        .read_json("usda-foundation-foods-essential.json")
        .map_err(|e| format!("failed to read USDA foods: {e}"))?;

    let food_name_substring = get_str_arg(args, "food_name_substring");
    let nutrient = get_str_arg(args, "nutrient");
    let max_results = get_u64_arg(args, "max_results").unwrap_or(50) as usize;

    let mut foods: Vec<&Food> = if let Some(ref needle) = food_name_substring {
        let needle_lower = needle.to_lowercase();
        foods_data.foods.iter().filter(|f| f.name.to_lowercase().contains(&needle_lower)).collect()
    } else {
        foods_data.foods.iter().collect()
    };

    if let Some(ref nutrient_name) = nutrient {
        foods.sort_by(|a, b| {
            let a_val = a.nutrients.get(nutrient_name).map(|n| n.amount).unwrap_or(0.0);
            let b_val = b.nutrients.get(nutrient_name).map(|n| n.amount).unwrap_or(0.0);
            b_val.partial_cmp(&a_val).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    let data: Vec<serde_json::Value> = foods.iter().take(max_results).map(|food| {
        let mut entry = serde_json::json!({
            "food_name": food.name,
            "food_category": food.category,
            "fdc_id": food.fdc_id,
        });
        for (nutrient_name, amount) in &food.nutrients {
            entry[nutrient_name] = serde_json::json!(amount.amount);
        }
        entry
    }).collect();

    let mut filters = serde_json::json!({});
    if let Some(n) = food_name_substring {
        filters["food_name_substring"] = serde_json::json!(n);
    }
    if let Some(n) = nutrient {
        filters["nutrient"] = serde_json::json!(n);
    }
    filters["max_results"] = serde_json::json!(max_results);

    Ok(build_response(&data, data.len(), filters))
}

fn find_severity<'a>(severities: &'a [HbSeverityRange], group: &str) -> Option<&'a HbSeverityRange> {
    if let Some(s) = severities.iter().find(|s| s.group == group) {
        return Some(s);
    }
    let prefix = group.rsplit_once('_').map(|(p, _)| p).unwrap_or(group);
    severities.iter().find(|s| s.group.starts_with(prefix))
}

fn query_who_hb_impl(loader: &DataLoader, args: &serde_json::Value) -> Result<String, String> {
    let hb: WhoHbThresholds = loader
        .read_json("who-hb-thresholds.json")
        .map_err(|e| format!("failed to read WHO Hb thresholds: {e}"))?;

    let sex = get_str_arg(args, "sex");
    let pregnant = get_bool_arg(args, "pregnant");
    let age_group = get_str_arg(args, "age_group");

    let thresholds: Vec<&HbDiagnosticThreshold> = hb.diagnostic_thresholds.iter().filter(|t| {
        if let Some(ref s) = sex {
            if &t.sex != s { return false; }
        }
        if let Some(p) = pregnant {
            if t.pregnant != p { return false; }
        }
        if let Some(ref ag) = age_group {
            let ag_lower = ag.to_lowercase();
            if !t.group.to_lowercase().contains(&ag_lower) { return false; }
        }
        true
    }).collect();

    let data: Vec<serde_json::Value> = thresholds.iter().map(|t| {
        let severity = find_severity(&hb.severity_classification, &t.group);
        let mut entry = serde_json::json!({
            "group": t.group,
            "sex": t.sex,
            "pregnant": t.pregnant,
            "diagnostic_threshold_g_per_l": t.hb_cutoff_g_per_l,
            "diagnostic_threshold_g_per_dl": t.hb_cutoff_g_per_dl,
            "severity_mild_low": severity.map(|s| serde_json::json!(s.mild_low)),
            "severity_mild_high": severity.map(|s| serde_json::json!(s.mild_high)),
            "severity_moderate_low": severity.map(|s| serde_json::json!(s.moderate_low)),
            "severity_moderate_high": severity.map(|s| serde_json::json!(s.moderate_high)),
            "severity_severe_below": severity.map(|s| serde_json::json!(s.severe_below)),
        });
        if let Some(ref note) = t.note {
            entry["note"] = serde_json::json!(note);
        }
        entry
    }).collect();

    let mut filters = serde_json::json!({});
    if let Some(s) = sex {
        filters["sex"] = serde_json::json!(s);
    }
    if let Some(p) = pregnant {
        filters["pregnant"] = serde_json::json!(p);
    }
    if let Some(ag) = age_group {
        filters["age_group"] = serde_json::json!(ag);
    }

    Ok(build_response(&data, data.len(), filters))
}

fn register_dri_query(
    registry: &mut ToolRegistry,
    loader: &DataLoader,
    name: &str,
    description: &str,
    input_schema: serde_json::Value,
    handler_fn: fn(&DataLoader, &serde_json::Value) -> Result<String, String>,
) {
    let l = loader.clone();
    registry.register(
        name,
        description,
        input_schema,
        Box::new(move |args: &serde_json::Value| -> Result<String, String> {
            handler_fn(&l, args)
        }),
    );
}

/// Register all query tools (Phase 1 DRI + Phase 2 USDA Foods/WHO Hb).
pub fn register_query_tools(registry: &mut ToolRegistry, loader: &DataLoader) {
    // Phase 1: DRI
    register_dri_query(
        registry,
        loader,
        "query_dri_minerals",
        "Query DRI mineral values by nutrient name with optional filters (group, sex, pregnant, breastfeeding)",
        serde_json::json!({
            "type": "object",
            "properties": {
                "nutrient": {"type": "string", "description": "Mineral name (required). Use describe_dri_minerals() for valid names."},
                "group": {"type": "string", "description": "Exact group key."},
                "sex": {"type": "string", "enum": ["male", "female"]},
                "pregnant": {"type": "boolean"},
                "breastfeeding": {"type": "boolean"}
            },
            "required": ["nutrient"]
        }),
        query_dri_minerals_impl,
    );

    register_dri_query(
        registry,
        loader,
        "query_dri_vitamins",
        "Query DRI vitamin values by nutrient name with optional filters (group, sex)",
        serde_json::json!({
            "type": "object",
            "properties": {
                "nutrient": {"type": "string", "description": "Vitamin name (required). Use describe_dri_vitamins() for valid names."},
                "group": {"type": "string", "description": "Exact group key."},
                "sex": {"type": "string", "enum": ["male", "female"]}
            },
            "required": ["nutrient"]
        }),
        query_dri_vitamins_impl,
    );

    register_dri_query(
        registry,
        loader,
        "query_dri_per_kg",
        "Query DRI per-kg values by nutrient name with optional group filter",
        serde_json::json!({
            "type": "object",
            "properties": {
                "nutrient": {"type": "string", "description": "Nutrient name (required). Use describe_dri_per_kg() for valid names."},
                "group": {"type": "string", "description": "Exact group key."}
            },
            "required": ["nutrient"]
        }),
        query_dri_per_kg_impl,
    );

    // Phase 2: USDA Foods
    register_dri_query(
        registry,
        loader,
        "query_usda_foods",
        "Query USDA Foundation Foods by name substring, nutrient sort, and max results",
        serde_json::json!({
            "type": "object",
            "properties": {
                "food_name_substring": {"type": "string", "description": "Case-insensitive substring search on food name."},
                "nutrient": {"type": "string", "description": "Nutrient name to sort by descending amount. Use describe_usda_foods() for valid names."},
                "max_results": {"type": "integer", "description": "Max results to return (default 50)."}
            },
            "required": []
        }),
        query_usda_foods_impl,
    );

    // Phase 2: WHO Hb thresholds
    register_dri_query(
        registry,
        loader,
        "query_who_hb",
        "Query WHO haemoglobin thresholds by sex, pregnant status, and age group substring",
        serde_json::json!({
            "type": "object",
            "properties": {
                "sex": {"type": "string", "enum": ["male", "female", "any"]},
                "pregnant": {"type": "boolean"},
                "age_group": {"type": "string", "description": "Substring match on diagnostic group name (e.g., 'children', 'trimester')."}
            },
            "required": []
        }),
        query_who_hb_impl,
    );
}
