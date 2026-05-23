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

/// Tool handler: takes arguments JSON, returns result JSON string.
/// Box<dyn Fn> allows handlers to capture DataLoader and other state.
pub type ToolFn = Box<dyn Fn(&serde_json::Value) -> Result<String, String> + Send + Sync>;

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
