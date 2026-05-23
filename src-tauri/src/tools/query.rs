use crate::data::DataLoader;
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
            if let Some(ref s) = sex {
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

/// Register all query tools for Phase 1 (DRI: minerals, vitamins, per-kg).
pub fn register_query_tools(registry: &mut ToolRegistry, loader: &DataLoader) {
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
}
