/// Describe tool placeholders — return not_implemented until phases 1-4.
/// Each function signature: fn(&serde_json::Value) -> Result<String, String>
pub fn describe_dri_minerals(_args: &serde_json::Value) -> Result<String, String> {
    Ok(r#"{"status": "not_implemented", "message": "Phase 1 task"}"#.to_string())
}

pub fn describe_dri_vitamins(_args: &serde_json::Value) -> Result<String, String> {
    Ok(r#"{"status": "not_implemented", "message": "Phase 1 task"}"#.to_string())
}

pub fn describe_dri_per_kg(_args: &serde_json::Value) -> Result<String, String> {
    Ok(r#"{"status": "not_implemented", "message": "Phase 1 task"}"#.to_string())
}

pub fn describe_usda_foods(_args: &serde_json::Value) -> Result<String, String> {
    Ok(r#"{"status": "not_implemented", "message": "Phase 2 task"}"#.to_string())
}

pub fn describe_who_hb(_args: &serde_json::Value) -> Result<String, String> {
    Ok(r#"{"status": "not_implemented", "message": "Phase 2 task"}"#.to_string())
}

pub fn describe_who_anaemia(_args: &serde_json::Value) -> Result<String, String> {
    Ok(r#"{"status": "not_implemented", "message": "Phase 3 task"}"#.to_string())
}

pub fn describe_who_bmi(_args: &serde_json::Value) -> Result<String, String> {
    Ok(r#"{"status": "not_implemented", "message": "Phase 3 task"}"#.to_string())
}

pub fn describe_who_diabetes(_args: &serde_json::Value) -> Result<String, String> {
    Ok(r#"{"status": "not_implemented", "message": "Phase 3 task"}"#.to_string())
}

pub fn describe_lab_ranges(_args: &serde_json::Value) -> Result<String, String> {
    Ok(r#"{"status": "not_implemented", "message": "Phase 4 task"}"#.to_string())
}
