use dietology_lib::tools::registry::{ToolCall, ToolRegistry};
use serde_json::json;

fn setup_query_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::query::register_query_tools(&mut registry, &loader);
    registry
}

fn call_query(registry: &ToolRegistry, name: &str, args: serde_json::Value) -> serde_json::Value {
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: name.to_string(),
        arguments: args,
    };
    let result = registry.dispatch(&call).unwrap();
    serde_json::from_str(&result.content).unwrap()
}

#[test]
fn test_query_dri_minerals_calcium_male() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_dri_minerals",
        json!({"nutrient": "Calcium", "sex": "male"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 6);
    assert_eq!(v["filters_applied"]["nutrient"], "Calcium");
    assert_eq!(v["filters_applied"]["sex"], "male");

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 6);

    let groups: Vec<&str> = data.iter().map(|g| g["group"].as_str().unwrap()).collect();
    assert!(groups.contains(&"male_9_13yr"));
    assert!(groups.contains(&"male_19_30yr"));
    assert!(groups.contains(&"male_gt70yr"));

    for entry in data {
        assert_eq!(entry["sex"], "male");
        assert_eq!(entry["unit"], "mg");
        assert!(entry["value"].is_number());
    }
}

#[test]
fn test_query_dri_minerals_iron_pregnant() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_dri_minerals",
        json!({"nutrient": "Iron", "pregnant": true}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 3);
    assert_eq!(v["filters_applied"]["nutrient"], "Iron");
    assert_eq!(v["filters_applied"]["pregnant"], true);

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 3);

    let groups: Vec<&str> = data.iter().map(|g| g["group"].as_str().unwrap()).collect();
    assert!(groups.contains(&"pregnant_14_18yr"));
    assert!(groups.contains(&"pregnant_19_30yr"));
    assert!(groups.contains(&"pregnant_31_50yr"));

    for entry in data {
        assert!(entry["group"].as_str().unwrap().starts_with("pregnant"));
        assert_eq!(entry["unit"], "mg");
    }
}

#[test]
fn test_query_dri_vitamins_folate_female() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_dri_vitamins",
        json!({"nutrient": "Folate", "sex": "female"}),
    );

    assert_eq!(v["status"], "ok");
    assert!(v["total_count"].as_u64().unwrap() >= 6);
    assert_eq!(v["filters_applied"]["nutrient"], "Folate");
    assert_eq!(v["filters_applied"]["sex"], "female");

    let data = v["data"].as_array().unwrap();
    assert!(data.len() >= 6);

    let groups: Vec<&str> = data.iter().map(|g| g["group"].as_str().unwrap()).collect();
    assert!(groups.contains(&"female_9_13yr"));
    assert!(groups.contains(&"female_19_70yr"));
    assert!(groups.contains(&"pregnant_19_50yr"));
    assert!(groups.contains(&"breastfeeding_19_50yr"));

    for entry in data {
        assert_eq!(entry["sex"], "female");
        assert_eq!(entry["unit"], "mcg DFE");
    }
}

#[test]
fn test_query_dri_vitamins_unknown_nutrient() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_dri_vitamins",
        json!({"nutrient": "Vitamin X"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 0);
    assert_eq!(v["filters_applied"]["nutrient"], "Vitamin X");
    let data = v["data"].as_array().unwrap();
    assert!(data.is_empty());
}

#[test]
fn test_query_dri_per_kg_calcium() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_dri_per_kg",
        json!({"nutrient": "Calcium"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 17);
    assert_eq!(v["filters_applied"]["nutrient"], "Calcium");

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 17);

    for entry in data {
        assert_eq!(entry["unit"], "mg/kg");
        assert!(entry["value"].is_number());
    }
}

#[test]
fn test_query_dri_minerals_calcium_all() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_dri_minerals",
        json!({"nutrient": "Calcium"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 22);
    assert_eq!(v["filters_applied"]["nutrient"], "Calcium");

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 22);

    // All 22 groups: infants, children, males(6), females(6), pregnant(3), breastfeeding(3)
    let groups: Vec<&str> = data.iter().map(|g| g["group"].as_str().unwrap()).collect();
    assert!(groups.contains(&"infants_0_6mo"));
    assert!(groups.contains(&"male_gt70yr"));
    assert!(groups.contains(&"female_gt70yr"));
    assert!(groups.contains(&"pregnant_14_18yr"));
    assert!(groups.contains(&"breastfeeding_31_50yr"));

    for entry in data {
        assert_eq!(entry["unit"], "mg");
        assert!(entry["value"].is_number());
    }
}

#[test]
fn test_query_dri_minerals_calcium_male_19_30yr() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_dri_minerals",
        json!({"nutrient": "Calcium", "group": "male_19_30yr"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 1);
    assert_eq!(v["filters_applied"]["nutrient"], "Calcium");
    assert_eq!(v["filters_applied"]["group"], "male_19_30yr");

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);

    let entry = &data[0];
    assert_eq!(entry["group"], "male_19_30yr");
    assert_eq!(entry["sex"], "male");
    assert_eq!(entry["value"], 1000.0);
    assert_eq!(entry["type"], "RDA");
    assert_eq!(entry["unit"], "mg");
    assert_eq!(entry["age_range"], "19\u{2013}30y");
}

// ---- Phase 2: USDA Foods ----

#[test]
fn test_query_usda_foods_apple() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_usda_foods",
        json!({"food_name_substring": "apple"}),
    );

    assert_eq!(v["status"], "ok");
    assert!(v["total_count"].as_u64().unwrap() >= 1);
    assert_eq!(v["filters_applied"]["food_name_substring"], "apple");

    let data = v["data"].as_array().unwrap();
    for entry in data {
        let name = entry["food_name"].as_str().unwrap().to_lowercase();
        assert!(name.contains("apple"), "food_name must contain 'apple': {name}");
    }
}

#[test]
fn test_query_usda_foods_sort_by_iron() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_usda_foods",
        json!({"nutrient": "Iron, Fe", "max_results": 5}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 5);
    assert_eq!(v["filters_applied"]["nutrient"], "Iron, Fe");
    assert_eq!(v["filters_applied"]["max_results"], 5);

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 5);

    // Verify sorted descending: first entry must have highest Iron
    let first_iron = data[0]["Iron, Fe"].as_f64().unwrap_or(0.0);
    let last_iron = data[4]["Iron, Fe"].as_f64().unwrap_or(0.0);
    assert!(first_iron >= last_iron, "first Iron {first_iron} >= last Iron {last_iron}");
}

#[test]
fn test_query_usda_foods_empty_filters() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_usda_foods",
        json!({}),
    );

    assert_eq!(v["status"], "ok");
    let total = v["total_count"].as_u64().unwrap();
    assert!(total <= 50, "default max_results=50, got {total}");
    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len() as u64, total);
}

#[test]
fn test_query_usda_foods_unknown_substring() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_usda_foods",
        json!({"food_name_substring": "xyznonexistent"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 0);
    let data = v["data"].as_array().unwrap();
    assert!(data.is_empty());
}

// ---- Phase 2: WHO Hb thresholds ----

#[test]
fn test_query_who_hb_children() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_who_hb",
        json!({"age_group": "children"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 4);
    assert_eq!(v["filters_applied"]["age_group"], "children");

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 4);

    let groups: Vec<&str> = data.iter().map(|g| g["group"].as_str().unwrap()).collect();
    assert!(groups.contains(&"children_6_23_months"));
    assert!(groups.contains(&"children_24_59_months"));
    assert!(groups.contains(&"children_5_11_years"));
    assert!(groups.contains(&"children_12_14_years"));

    for entry in data {
        assert_eq!(entry["sex"], "any");
        assert!(!entry["pregnant"].as_bool().unwrap());
        assert!(entry["diagnostic_threshold_g_per_l"].is_number());
        assert!(entry["diagnostic_threshold_g_per_dl"].is_number());
        assert!(entry["severity_mild_low"].is_number());
        assert!(entry["severity_mild_high"].is_number());
        assert!(entry["severity_moderate_low"].is_number());
        assert!(entry["severity_moderate_high"].is_number());
        assert!(entry["severity_severe_below"].is_number());
    }
}

#[test]
fn test_query_who_hb_pregnant() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_who_hb",
        json!({"pregnant": true}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 3);
    assert_eq!(v["filters_applied"]["pregnant"], true);

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 3);

    let groups: Vec<&str> = data.iter().map(|g| g["group"].as_str().unwrap()).collect();
    assert!(groups.contains(&"pregnant_first_trimester"));
    assert!(groups.contains(&"pregnant_second_trimester"));
    assert!(groups.contains(&"pregnant_third_trimester"));

    for entry in data {
        assert_eq!(entry["sex"], "female");
        assert!(entry["pregnant"].as_bool().unwrap());
        assert!(entry["diagnostic_threshold_g_per_l"].is_number());
    }
}

#[test]
fn test_query_who_hb_male() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_who_hb",
        json!({"sex": "male"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 1);
    assert_eq!(v["filters_applied"]["sex"], "male");

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    let entry = &data[0];
    assert_eq!(entry["group"], "men_15_plus");
    assert_eq!(entry["sex"], "male");
    assert_eq!(entry["diagnostic_threshold_g_per_l"], 130.0);
    assert_eq!(entry["diagnostic_threshold_g_per_dl"], 13.0);

    // Severity fields via find_severity fallback (men_15_plus → men_15_65)
    assert!(entry["severity_mild_low"].is_number());
    assert!(entry["severity_mild_high"].is_number());
    assert!(entry["severity_moderate_low"].is_number());
    assert!(entry["severity_moderate_high"].is_number());
    assert!(entry["severity_severe_below"].is_number());
}

#[test]
fn test_query_who_hb_all() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_who_hb",
        json!({}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 9);

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 9);
}

// ---- Phase 3: WHO GHO epidemiology ----

#[test]
fn test_query_who_anaemia_rus_2019_total() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_who_anaemia",
        json!({"country_code": "RUS", "year": 2019, "severity": "SEVERITY_TOTAL"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 1);
    assert_eq!(v["filters_applied"]["country_code"], "RUS");
    assert_eq!(v["filters_applied"]["year"], 2019);
    assert_eq!(v["filters_applied"]["severity"], "SEVERITY_TOTAL");

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    let entry = &data[0];
    assert_eq!(entry["country_code"], "RUS");
    assert_eq!(entry["year"], 2019);
    assert_eq!(entry["severity"], "SEVERITY_TOTAL");
    assert!(entry["value"].is_number());
    assert!(entry["low"].is_number());
    assert!(entry["high"].is_number());
    assert_eq!(entry["parent_region"], "Europe");
    assert_eq!(entry["parent_region_code"], "EUR");
}

#[test]
fn test_query_who_anaemia_all_empty() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_who_anaemia",
        json!({}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 20950);
    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 20950);
}

#[test]
fn test_query_who_bmi_afg_2020() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_who_bmi",
        json!({"country_code": "AFG", "year": 2020}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 3);
    assert_eq!(v["filters_applied"]["country_code"], "AFG");
    assert_eq!(v["filters_applied"]["year"], 2020);

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 3);

    let sexes: Vec<&str> = data.iter().map(|r| r["sex"].as_str().unwrap()).collect();
    assert!(sexes.contains(&"SEX_BTSX"));
    assert!(sexes.contains(&"SEX_MLE"));
    assert!(sexes.contains(&"SEX_FMLE"));

    for entry in data {
        assert_eq!(entry["country_code"], "AFG");
        assert_eq!(entry["year"], 2020);
        assert!(entry["value"].is_number());
        assert!(entry["low"].is_number());
        assert!(entry["high"].is_number());
    }
}

#[test]
fn test_query_who_diabetes_afg_2022_fmle_30plus() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_who_diabetes",
        json!({"country_code": "AFG", "year": 2022, "sex": "SEX_FMLE", "agegroup": "AGEGROUP_YEARS30-PLUS"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 1);
    assert_eq!(v["filters_applied"]["country_code"], "AFG");
    assert_eq!(v["filters_applied"]["year"], 2022);
    assert_eq!(v["filters_applied"]["sex"], "SEX_FMLE");
    assert_eq!(v["filters_applied"]["agegroup"], "AGEGROUP_YEARS30-PLUS");

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    let entry = &data[0];
    assert_eq!(entry["country_code"], "AFG");
    assert_eq!(entry["year"], 2022);
    assert_eq!(entry["sex"], "SEX_FMLE");
    assert_eq!(entry["agegroup"], "AGEGROUP_YEARS30-PLUS");
    assert!(entry["value"].is_number());
    assert!(entry["low"].is_number());
    assert!(entry["high"].is_number());
}

#[test]
fn test_query_who_anaemia_unknown_country() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_who_anaemia",
        json!({"country_code": "XYZ"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 0);
    let data = v["data"].as_array().unwrap();
    assert!(data.is_empty());
}

// ---- Phase 4: Lab reference ranges ----

#[test]
fn test_query_lab_ranges_ferritin() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_lab_ranges",
        json!({"test_name_substring": "ferritin"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["filters_applied"]["test_name_substring"], "ferritin");

    let data = v["data"].as_array().unwrap();
    assert!(!data.is_empty(), "should find at least one ferritin test");
    for entry in data {
        let test_name = entry["test_name"].as_str().unwrap();
        assert!(
            test_name.to_lowercase().contains("ferritin"),
            "expected 'ferritin' in test name"
        );
    }
}

#[test]
fn test_query_lab_ranges_thyroid_category() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_lab_ranges",
        json!({"category": "thyroid"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 13);
    assert_eq!(v["filters_applied"]["category"], "thyroid");

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 13);
    for entry in data {
        assert_eq!(entry["category"].as_str().unwrap(), "thyroid");
    }
}

#[test]
fn test_query_lab_ranges_both_filters() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_lab_ranges",
        json!({"test_name_substring": "ft3", "category": "thyroid"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["filters_applied"]["test_name_substring"], "ft3");
    assert_eq!(v["filters_applied"]["category"], "thyroid");

    let data = v["data"].as_array().unwrap();
    assert!(!data.is_empty(), "should find ft3 in thyroid category");
    for entry in data {
        assert_eq!(entry["category"].as_str().unwrap(), "thyroid");
        assert!(entry["test_name"].as_str().unwrap().to_lowercase().contains("ft3"));
    }
}

#[test]
fn test_query_lab_ranges_empty() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_lab_ranges",
        json!({}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 254);

    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 254);
}

#[test]
fn test_query_lab_ranges_not_found() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_lab_ranges",
        json!({"test_name_substring": "xyznonexistent"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 0);
    let data = v["data"].as_array().unwrap();
    assert!(data.is_empty());
}

#[test]
fn test_query_lab_ranges_wrong_case_category() {
    let registry = setup_query_registry();
    let v = call_query(
        &registry,
        "query_lab_ranges",
        json!({"category": "Thyroid"}),
    );

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_count"], 0);
    let data = v["data"].as_array().unwrap();
    assert!(data.is_empty());
}
