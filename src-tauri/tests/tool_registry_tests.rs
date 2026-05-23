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

// ============ Phase 1: DRI describe tools return real enum values ============

#[test]
fn test_describe_dri_minerals_returns_enums() {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: "describe_dri_minerals".to_string(),
        arguments: json!({}),
    };
    let result = registry.dispatch(&call).unwrap();
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert_eq!(v["status"], "ok");
    assert!(!v["nutrients"].as_array().unwrap().is_empty());
    assert!(!v["groups"].as_array().unwrap().is_empty());
    assert!(!v["sexes"].as_array().unwrap().is_empty());
    assert_eq!(v["total_groups"], 254);
}

#[test]
fn test_describe_dri_vitamins_returns_enums() {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: "describe_dri_vitamins".to_string(),
        arguments: json!({}),
    };
    let result = registry.dispatch(&call).unwrap();
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_groups"], 154);
}

#[test]
fn test_describe_dri_per_kg_returns_enums() {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: "describe_dri_per_kg".to_string(),
        arguments: json!({}),
    };
    let result = registry.dispatch(&call).unwrap();
    let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_groups"], 51);
    assert_eq!(v["unit"], "mg/kg");
    assert!(v["note"].as_str().unwrap().contains("body weight"));
}

#[test]
fn test_describe_dri_nutrients_have_expected_entries() {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);

    // Minerals: 14 nutrients
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: "describe_dri_minerals".to_string(),
        arguments: json!({}),
    };
    let v: serde_json::Value =
        serde_json::from_str(&registry.dispatch(&call).unwrap().content).unwrap();
    let names: Vec<&str> = v["nutrients"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert!(names.contains(&"Calcium"));
    assert!(names.contains(&"Iron"));
    assert!(names.contains(&"Zinc"));
    assert_eq!(names.len(), 14);

    // Vitamins: 11 nutrients
    let call = ToolCall {
        id: "call_2".to_string(),
        r#type: "tool_use".to_string(),
        name: "describe_dri_vitamins".to_string(),
        arguments: json!({}),
    };
    let v: serde_json::Value =
        serde_json::from_str(&registry.dispatch(&call).unwrap().content).unwrap();
    let names: Vec<&str> = v["nutrients"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert!(names.contains(&"Vitamin D"));
    assert!(names.contains(&"Folate"));
    assert_eq!(names.len(), 11);

    // Per-kg: 3 nutrients
    let call = ToolCall {
        id: "call_3".to_string(),
        r#type: "tool_use".to_string(),
        name: "describe_dri_per_kg".to_string(),
        arguments: json!({}),
    };
    let v: serde_json::Value =
        serde_json::from_str(&registry.dispatch(&call).unwrap().content).unwrap();
    let names: Vec<&str> = v["nutrients"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert!(names.contains(&"Calcium"));
    assert!(names.contains(&"Magnesium"));
    assert_eq!(names.len(), 3);
}

// ============ Phase 2+ placeholders still return not_implemented ============

#[test]
fn test_phase2_tool_returns_not_implemented() {
    let mut registry = ToolRegistry::new();
    let loader = dietology_lib::data::DataLoader::for_development();
    dietology_lib::tools::describe::register_describe_tools(&mut registry, &loader);
    let call = ToolCall {
        id: "call_1".to_string(),
        r#type: "tool_use".to_string(),
        name: "describe_usda_foods".to_string(),
        arguments: json!({}),
    };
    let result = registry.dispatch(&call).unwrap();
    assert!(result.content.contains("not_implemented"));
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
