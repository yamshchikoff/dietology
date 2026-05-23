use std::sync::Arc;

use crate::tools::registry::{ToolCall, ToolRegistry};

use super::types::{ApiRequest, ApiResponse, ContentBlock, LlmError, Message};

pub struct LlmClient {
    pub api_base_url: String,
    pub api_key: String,
    pub model: String,
    pub http: reqwest::Client,
    pub registry: Arc<ToolRegistry>,
    pub max_tokens: u32,
    pub max_tool_rounds: u8,
}

impl LlmClient {
    pub fn new(registry: Arc<ToolRegistry>) -> Result<Self, LlmError> {
        let api_key =
            std::env::var("DEEPSEEK_API_KEY").map_err(|_| LlmError::MissingApiKey)?;
        let api_base_url = std::env::var("DEEPSEEK_API_BASE")
            .unwrap_or_else(|_| "https://api.deepseek.com".into());
        let model =
            std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-chat".into());

        let http = reqwest::Client::new();

        Ok(Self {
            api_base_url,
            api_key,
            model,
            http,
            registry,
            max_tokens: 4096,
            max_tool_rounds: 10,
        })
    }

    pub async fn call_api(
        &self,
        messages: &[Message],
        system: &str,
    ) -> Result<ApiResponse, LlmError> {
        let url = format!("{}/v1/messages", self.api_base_url);
        let tools = self.registry.definitions();

        let request = ApiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: system.to_string(),
            messages: messages.to_vec(),
            tools,
        };

        let response = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".into());
            return Err(LlmError::Api {
                status: status.as_u16(),
                body,
            });
        }

        response
            .json::<ApiResponse>()
            .await
            .map_err(|e| LlmError::Parse(e.to_string()))
    }

    pub fn extract_tool_uses<'a>(&self, response: &'a ApiResponse) -> Vec<&'a ContentBlock> {
        response
            .content
            .iter()
            .filter(|block| matches!(block, ContentBlock::ToolUse { .. }))
            .collect()
    }

    pub fn dispatch_tool(&self, tool_use: &ContentBlock) -> Result<String, LlmError> {
        match tool_use {
            ContentBlock::ToolUse { id, name, input } => {
                let tool_call = ToolCall {
                    id: id.clone(),
                    r#type: "tool_use".into(),
                    name: name.clone(),
                    arguments: input.clone(),
                };
                self.registry
                    .dispatch(&tool_call)
                    .map(|result| result.content)
                    .map_err(LlmError::ToolDispatch)
            }
            _ => Err(LlmError::ToolDispatch(
                "extract_tool_uses should filter non-ToolUse blocks".into(),
            )),
        }
    }
}
