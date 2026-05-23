use serde::{Deserialize, Serialize};

/// Anthropic-compatible tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Represents a tool invocation from the LLM
#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(default = "default_tool_type")]
    pub r#type: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

fn default_tool_type() -> String {
    "tool_use".to_string()
}

/// Result returned to the LLM
#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub content: String,
}

/// Tool function signature: takes arguments JSON, returns result JSON string
pub type ToolFn = fn(&serde_json::Value) -> Result<String, String>;

struct ToolDefEntry {
    definition: ToolDefinition,
    handler: ToolFn,
}

/// Registry of all registered tools
pub struct ToolRegistry {
    tools: Vec<ToolDefEntry>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register(
        &mut self,
        name: &str,
        description: &str,
        input_schema: serde_json::Value,
        handler: ToolFn,
    ) {
        self.tools.push(ToolDefEntry {
            definition: ToolDefinition {
                name: name.to_string(),
                description: description.to_string(),
                input_schema,
            },
            handler,
        });
    }

    /// Return all tool definitions (for the `tools` parameter in LLM API calls)
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|e| e.definition.clone()).collect()
    }

    /// Dispatch a tool call and return the result
    pub fn dispatch(&self, call: &ToolCall) -> Result<ToolResult, String> {
        for entry in &self.tools {
            if entry.definition.name == call.name {
                let content = (entry.handler)(&call.arguments)?;
                return Ok(ToolResult { content });
            }
        }
        Err(format!("unknown tool: {}", call.name))
    }
}

/// Register all 9 describe tools as placeholders (implemented in phases 1-4)
pub fn register_describe_tools(registry: &mut ToolRegistry) {
    let empty_schema = serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    });

    macro_rules! register {
        ($registry:expr, $name:literal, $desc:literal, $handler:expr) => {
            $registry.register($name, $desc, empty_schema.clone(), $handler)
        };
    }

    use crate::tools::describe;

    register!(registry, "describe_dri_minerals",
        "Return valid enum values for DRI minerals dataset filters (nutrients, groups, sexes)",
        describe::describe_dri_minerals);
    register!(registry, "describe_dri_vitamins",
        "Return valid enum values for DRI vitamins dataset filters (nutrients, groups, sexes)",
        describe::describe_dri_vitamins);
    register!(registry, "describe_dri_per_kg",
        "Return valid enum values for DRI per-kg dataset filters (nutrients, groups, unit)",
        describe::describe_dri_per_kg);
    register!(registry, "describe_usda_foods",
        "Return valid enum values for USDA foods dataset filters (nutrients, food_categories)",
        describe::describe_usda_foods);
    register!(registry, "describe_who_hb",
        "Return valid enum values for WHO Hb thresholds (diagnostic_groups, severity_levels)",
        describe::describe_who_hb);
    register!(registry, "describe_who_anaemia",
        "Return valid enum values for WHO anaemia data (countries, years, severities)",
        describe::describe_who_anaemia);
    register!(registry, "describe_who_bmi",
        "Return valid enum values for WHO BMI data (countries, years, sexes, agegroups)",
        describe::describe_who_bmi);
    register!(registry, "describe_who_diabetes",
        "Return valid enum values for WHO diabetes data (countries, years, sexes, agegroups)",
        describe::describe_who_diabetes);
    register!(registry, "describe_lab_ranges",
        "Return valid enum values for lab reference ranges (categories, tests)",
        describe::describe_lab_ranges);
}
