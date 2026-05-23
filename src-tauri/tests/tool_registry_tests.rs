use dietology_lib::tools::registry::{ToolCall, ToolRegistry};
use serde_json::json;

#[test]
fn test_registry_new_is_empty() {
    let registry = ToolRegistry::new();
    let defs = registry.definitions();
    assert!(defs.is_empty(), "new registry should have no tools");
}

#[test]
fn test_register_adds_tool() {
    let mut registry = ToolRegistry::new();
    registry.register(
        "test_tool",
        "A test tool",
        json!({"type": "object", "properties": {}, "required": []}),
        Box::new(|_args| Ok(r#"{"ok": true}"#.to_string())),
    );
    let defs = registry.definitions();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].name, "test_tool");
    assert_eq!(defs[0].description, "A test tool");
}

#[test]
fn test_dispatch_calls_correct_handler() {
    let mut registry = ToolRegistry::new();
    registry.register(
        "echo",
        "Echoes the input",
        json!({"type": "object", "properties": {}, "required": []}),
        Box::new(|args| Ok(args.to_string())),
    );
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: "echo".to_string(),
        arguments: json!({"msg": "hello"}),
    };
    let result = registry.dispatch(&call).unwrap();
    assert!(result.content.contains("hello"));
}

#[test]
fn test_dispatch_unknown_tool_returns_error() {
    let registry = ToolRegistry::new();
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: "nonexistent".to_string(),
        arguments: json!({}),
    };
    let result = registry.dispatch(&call);
    assert!(result.is_err(), "unknown tool should return error");
    assert!(result.unwrap_err().contains("unknown tool"));
}

#[test]
fn test_register_describe_tools_registers_nine() {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);
    let defs = registry.definitions();
    assert_eq!(defs.len(), 9, "expected 9 describe tools");

    let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"describe_dri_minerals"));
    assert!(names.contains(&"describe_dri_vitamins"));
    assert!(names.contains(&"describe_dri_per_kg"));
    assert!(names.contains(&"describe_usda_foods"));
    assert!(names.contains(&"describe_who_hb"));
    assert!(names.contains(&"describe_who_anaemia"));
    assert!(names.contains(&"describe_who_bmi"));
    assert!(names.contains(&"describe_who_diabetes"));
    assert!(names.contains(&"describe_lab_ranges"));
}

fn call_describe(registry: &ToolRegistry, name: &str) -> serde_json::Value {
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: name.to_string(),
        arguments: json!({}),
    };
    let result = registry.dispatch(&call).unwrap();
    serde_json::from_str(&result.content).unwrap()
}

#[test]
fn test_describe_dri_minerals() {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);

    let v = call_describe(&registry, "describe_dri_minerals");

    assert_eq!(v["status"], "ok");
    assert_eq!(v["nutrients"].as_array().unwrap().len(), 14);
    assert!(v["nutrients"].as_array().unwrap().iter().any(|n| n == "Calcium"));
    assert!(v["nutrients"].as_array().unwrap().iter().any(|n| n == "Zinc"));
    assert!(v["groups"].as_array().unwrap().len() > 0);
    assert_eq!(v["sexes"].as_array().unwrap().len(), 2);
    assert!(v["sexes"].as_array().unwrap().iter().any(|s| s == "male"));
    assert!(v["sexes"].as_array().unwrap().iter().any(|s| s == "female"));
    assert_eq!(v["total_groups"], 254);
}

#[test]
fn test_describe_dri_vitamins() {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);

    let v = call_describe(&registry, "describe_dri_vitamins");

    assert_eq!(v["status"], "ok");
    assert_eq!(v["nutrients"].as_array().unwrap().len(), 11);
    assert!(v["nutrients"].as_array().unwrap().iter().any(|n| n == "Folate"));
    assert!(v["nutrients"].as_array().unwrap().iter().any(|n| n == "Vitamin C"));
    assert!(v["groups"].as_array().unwrap().len() > 0);
    assert_eq!(v["sexes"].as_array().unwrap().len(), 2);
    assert!(v["sexes"].as_array().unwrap().iter().any(|s| s == "male"));
    assert!(v["sexes"].as_array().unwrap().iter().any(|s| s == "female"));
    assert_eq!(v["total_groups"], 154);
}

#[test]
fn test_describe_dri_per_kg() {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);

    let v = call_describe(&registry, "describe_dri_per_kg");

    assert_eq!(v["status"], "ok");
    assert_eq!(v["nutrients"].as_array().unwrap().len(), 3);
    assert!(v["nutrients"].as_array().unwrap().iter().any(|n| n == "Calcium"));
    assert!(v["groups"].as_array().unwrap().len() > 0);
    assert_eq!(v["total_groups"], 51);
    assert_eq!(v["unit"], "mg/kg");
    assert!(!v["note"].as_str().unwrap().is_empty());
}

#[test]
fn test_describe_usda_foods() {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);

    let v = call_describe(&registry, "describe_usda_foods");

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_foods"], 363);
    assert_eq!(v["nutrients"].as_array().unwrap().len(), 25);
    assert!(v["nutrients"].as_array().unwrap().iter().any(|n| n == "Calcium, Ca"));
    assert!(v["nutrients"].as_array().unwrap().iter().any(|n| n == "Protein"));
    assert_eq!(v["food_categories"].as_array().unwrap().len(), 19);
    assert!(v["food_categories"].as_array().unwrap().iter().any(|c| c == "Dairy and Egg Products"));
}

#[test]
fn test_describe_who_hb() {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);

    let v = call_describe(&registry, "describe_who_hb");

    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_thresholds"], 9);
    assert_eq!(v["diagnostic_groups"].as_array().unwrap().len(), 9);
    assert!(v["diagnostic_groups"].as_array().unwrap().iter().any(|g| g == "children_6_23_months"));
    assert!(v["diagnostic_groups"].as_array().unwrap().iter().any(|g| g == "pregnant_first_trimester"));
    assert_eq!(v["severity_levels"].as_array().unwrap().len(), 4);
    assert!(v["severity_levels"].as_array().unwrap().iter().any(|s| s == "mild"));
    assert!(v["severity_levels"].as_array().unwrap().iter().any(|s| s == "severe"));
    assert_eq!(v["sexes"].as_array().unwrap().len(), 3);
    assert!(v["sexes"].as_array().unwrap().iter().any(|s| s == "male"));
    assert!(v["sexes"].as_array().unwrap().iter().any(|s| s == "female"));
    assert!(v["sexes"].as_array().unwrap().iter().any(|s| s == "any"));
    assert_eq!(v["pregnant_options"].as_array().unwrap().len(), 2);
    assert!(v["pregnant_options"].as_array().unwrap().iter().any(|p| p == true));
    assert!(v["pregnant_options"].as_array().unwrap().iter().any(|p| p == false));
}

#[test]
fn test_tool_definition_has_input_schema() {
    let mut registry = ToolRegistry::new();
    let schema =
        json!({"type": "object", "properties": {"x": {"type": "number"}}, "required": ["x"]});
    registry.register(
        "with_schema",
        "Has schema",
        schema.clone(),
        Box::new(|_| Ok("ok".into())),
    );
    let defs = registry.definitions();
    assert_eq!(defs[0].input_schema, schema);
}
