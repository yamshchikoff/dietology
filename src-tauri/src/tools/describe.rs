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

fn build_usda_foods_describe(foods: &UsdaFoods, category: Option<&str>) -> serde_json::Value {
    if let Some(cat) = category {
        let matching: Vec<serde_json::Value> = foods
            .foods
            .iter()
            .filter(|f| f.category == cat)
            .map(|f| {
                serde_json::json!({
                    "food_name": f.name,
                    "fdc_id": f.fdc_id,
                })
            })
            .collect();

        return serde_json::json!({
            "status": "ok",
            "category": cat,
            "foods": matching,
            "count": matching.len(),
        });
    }

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

fn build_lab_ranges_describe(lr: &LabReferenceRanges, category: Option<&str>) -> serde_json::Value {
    if let Some(cat) = category {
        let matching: Vec<serde_json::Value> = lr
            .ranges
            .iter()
            .filter(|r| r.category == cat)
            .map(|r| {
                let mut entry = serde_json::json!({
                    "test_name": r.test,
                    "unit": r.unit,
                });
                if let Some(ref lower) = r.lower {
                    entry["lower"] = serde_json::json!(lower);
                }
                if let Some(ref upper) = r.upper {
                    entry["upper"] = serde_json::json!(upper);
                }
                if let Some(ref rt) = r.range_type {
                    if !rt.is_empty() {
                        entry["range_type"] = serde_json::json!(rt);
                    }
                }
                entry
            })
            .collect();

        return serde_json::json!({
            "status": "ok",
            "category": cat,
            "tests": matching,
            "count": matching.len(),
        });
    }

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
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "category": {"type": "string", "description": "Optional: get all food names in this category. Call without args first to see available categories."}
            },
            "required": []
        });
        registry.register(
            "describe_usda_foods",
            "Navigate USDA foods: without args returns nutrients + food_categories index; with category returns all food_name + fdc_id entries in that category",
            schema,
            Box::new(move |args: &serde_json::Value| -> Result<String, String> {
                let foods: UsdaFoods = l
                    .read_json("usda-foundation-foods-essential.json")
                    .map_err(|e| format!("failed to read USDA foods: {e}"))?;
                let cat = args.get("category").and_then(|v| v.as_str());
                Ok(build_usda_foods_describe(&foods, cat).to_string())
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
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "category": {"type": "string", "description": "Optional: get all test names in this category. Call without args first to see available categories."}
            },
            "required": []
        });
        registry.register(
            "describe_lab_ranges",
            "Navigate lab reference ranges: without args returns categories index with test counts; with category returns all test_name + reference values in that category",
            schema,
            Box::new(move |args: &serde_json::Value| -> Result<String, String> {
                let lr: LabReferenceRanges = l
                    .read_json("lab-reference-ranges.json")
                    .map_err(|e| format!("failed to read lab reference ranges: {e}"))?;
                let cat = args.get("category").and_then(|v| v.as_str());
                Ok(build_lab_ranges_describe(&lr, cat).to_string())
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::datasets::{Food, LabRange, LabReferenceRanges, NutrientAmount, UsdaFoods};
    use std::collections::HashMap;

    fn make_test_foods() -> UsdaFoods {
        UsdaFoods {
            foods: vec![
                Food {
                    name: "Chicken, breast, boneless, skinless, raw".into(),
                    category: "Poultry Products".into(),
                    fdc_id: 171108,
                    nutrients: HashMap::from([
                        ("Energy".into(), NutrientAmount { amount: 120.0, unit: "kcal".into() }),
                        ("Protein".into(), NutrientAmount { amount: 22.5, unit: "g".into() }),
                    ]),
                },
                Food {
                    name: "Apple, raw".into(),
                    category: "Fruits and Fruit Juices".into(),
                    fdc_id: 171688,
                    nutrients: HashMap::from([
                        ("Energy".into(), NutrientAmount { amount: 52.0, unit: "kcal".into() }),
                    ]),
                },
            ],
        }
    }

    fn make_test_lab_ranges() -> LabReferenceRanges {
        LabReferenceRanges {
            ranges: vec![
                LabRange {
                    category: "thyroid".into(),
                    test: "adults – optimal range".into(),
                    range_type: None,
                    lower: Some("0.3, 0.5".into()),
                    upper: Some("2.0, 3.0".into()),
                    unit: "mIU/L".into(),
                },
                LabRange {
                    category: "thyroid".into(),
                    test: "free thyroxine (ft4)".into(),
                    range_type: None,
                    lower: None,
                    upper: Some("0.7, 0.8".into()),
                    unit: "ng/dL".into(),
                },
                LabRange {
                    category: "lipids".into(),
                    test: "total cholesterol".into(),
                    range_type: Some("desirable".into()),
                    lower: None,
                    upper: Some("200".into()),
                    unit: "mg/dL".into(),
                },
            ],
        }
    }

    #[test]
    fn usda_index() {
        let foods = make_test_foods();
        let result = build_usda_foods_describe(&foods, None);
        assert_eq!(result["status"], "ok");
        assert_eq!(result["total_foods"], 2);
        let categories: Vec<&str> = result["food_categories"]
            .as_array().unwrap().iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(categories.contains(&"Poultry Products"));
        assert!(categories.contains(&"Fruits and Fruit Juices"));
        let nutrients: Vec<&str> = result["nutrients"]
            .as_array().unwrap().iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(nutrients.contains(&"Energy"));
        assert!(nutrients.contains(&"Protein"));
    }

    #[test]
    fn usda_drilldown() {
        let foods = make_test_foods();
        let result = build_usda_foods_describe(&foods, Some("Poultry Products"));
        assert_eq!(result["status"], "ok");
        assert_eq!(result["category"], "Poultry Products");
        assert_eq!(result["count"], 1);
        let food_list = result["foods"].as_array().unwrap();
        assert_eq!(food_list.len(), 1);
        assert_eq!(food_list[0]["food_name"], "Chicken, breast, boneless, skinless, raw");
        assert_eq!(food_list[0]["fdc_id"], 171108);
    }

    #[test]
    fn lab_index() {
        let lr = make_test_lab_ranges();
        let result = build_lab_ranges_describe(&lr, None);
        assert_eq!(result["status"], "ok");
        assert_eq!(result["total_tests"], 3);
        let categories: Vec<serde_json::Value> = result["categories"]
            .as_array().unwrap().iter()
            .map(|v| v.clone())
            .collect();
        let thyroid = categories.iter().find(|v| v["name"] == "thyroid").unwrap();
        assert_eq!(thyroid["count"], 2);
        let lipids = categories.iter().find(|v| v["name"] == "lipids").unwrap();
        assert_eq!(lipids["count"], 1);
    }

    #[test]
    fn lab_drilldown() {
        let lr = make_test_lab_ranges();
        let result = build_lab_ranges_describe(&lr, Some("thyroid"));
        assert_eq!(result["status"], "ok");
        assert_eq!(result["category"], "thyroid");
        assert_eq!(result["count"], 2);
        let tests = result["tests"].as_array().unwrap();
        let names: Vec<&str> = tests.iter().map(|t| t["test_name"].as_str().unwrap()).collect();
        assert!(names.contains(&"adults – optimal range"));
        assert!(names.contains(&"free thyroxine (ft4)"));
    }
}
