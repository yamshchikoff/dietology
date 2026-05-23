use crate::data::DataLoader;
use crate::tools::registry::{ToolFn, ToolRegistry};

fn placeholder(loader: DataLoader, phase: &'static str) -> ToolFn {
    Box::new(move |_args: &serde_json::Value| -> Result<String, String> {
        let _ = &loader;
        Ok(format!(
            r#"{{"status": "not_implemented", "message": "Phase {} task"}}"#,
            phase
        ))
    })
}

/// Register all 9 describe tools. Each handler captures a DataLoader clone
/// and will read JSON to extract enum values when implemented.
pub fn register_describe_tools(registry: &mut ToolRegistry, loader: &DataLoader) {
    let empty_schema = serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    });

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

    reg!("describe_dri_minerals",
        "Return valid enum values for DRI minerals dataset filters (nutrients, groups, sexes)",
        "1");
    reg!("describe_dri_vitamins",
        "Return valid enum values for DRI vitamins dataset filters (nutrients, groups, sexes)",
        "1");
    reg!("describe_dri_per_kg",
        "Return valid enum values for DRI per-kg dataset filters (nutrients, groups, unit)",
        "1");
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
