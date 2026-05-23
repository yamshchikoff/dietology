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
