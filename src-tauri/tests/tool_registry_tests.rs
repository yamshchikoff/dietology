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
        |_args| Ok(r#"{"ok": true}"#.to_string()),
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
        |args| Ok(args.to_string()),
    );
    let call = ToolCall {
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
    dietology_lib::tools::registry::register_describe_tools(&mut registry);
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

#[test]
fn test_describe_tool_returns_not_implemented() {
    let mut registry = ToolRegistry::new();
    dietology_lib::tools::registry::register_describe_tools(&mut registry);
    let call = ToolCall {
        name: "describe_dri_minerals".to_string(),
        arguments: json!({}),
    };
    let result = registry.dispatch(&call).unwrap();
    assert!(
        result.content.contains("not_implemented"),
        "describe placeholder should return not_implemented status"
    );
}

#[test]
fn test_tool_definition_has_input_schema() {
    let mut registry = ToolRegistry::new();
    let schema = json!({"type": "object", "properties": {"x": {"type": "number"}}, "required": ["x"]});
    registry.register("with_schema", "Has schema", schema.clone(), |_| Ok("ok".into()));
    let defs = registry.definitions();
    assert_eq!(defs[0].input_schema, schema);
}
