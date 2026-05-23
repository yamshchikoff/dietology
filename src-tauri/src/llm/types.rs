use serde::{Deserialize, Serialize};

use crate::tools::registry::ToolDefinition;

// ---- Content Blocks ----

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

// ---- Message ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Vec<ContentBlock>,
}

// ---- API Request ----

#[derive(Debug, Clone, Serialize)]
pub struct ApiRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
}

// ---- API Response ----

#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: String,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// ---- LLM Response ----

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub messages: Vec<Message>,
    pub final_text: String,
    pub visualization_json: Option<serde_json::Value>,
    pub usage: Usage,
}

// ---- LLM Error ----

#[derive(Debug)]
pub enum LlmError {
    Network(String),
    Api { status: u16, body: String },
    Parse(String),
    ToolDispatch(String),
    MaxToolRounds(u8),
    MissingApiKey,
}
