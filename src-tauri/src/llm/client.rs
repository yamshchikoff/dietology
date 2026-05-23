use std::sync::Arc;

use crate::tools::registry::{ToolCall, ToolRegistry};

use super::types::{ApiRequest, ApiResponse, ContentBlock, LlmError, LlmResponse, Message, Usage};

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

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| LlmError::Network(e.to_string()))?;

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

    /// Главный entry point: пользовательское сообщение → полный ответ с tool dispatch.
    ///
    /// Принимает `messages` как in/out параметр — накапливает историю диалога.
    /// Возвращает `LlmResponse` с финальным текстом и суммарным использованием токенов.
    pub async fn chat(
        &self,
        messages: &mut Vec<Message>,
        system_prompt: &str,
    ) -> Result<LlmResponse, LlmError> {
        let mut total_usage = Usage {
            input_tokens: 0,
            output_tokens: 0,
        };

        for _round in 0..self.max_tool_rounds {
            let response = self.call_api(messages, system_prompt).await?;

            total_usage.input_tokens += response.usage.input_tokens;
            total_usage.output_tokens += response.usage.output_tokens;

            messages.push(Message {
                role: "assistant".into(),
                content: response.content.clone(),
            });

            match response.stop_reason.as_str() {
                "end_turn" => {
                    let final_text = response
                        .content
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    return Ok(LlmResponse {
                        messages: messages.clone(),
                        final_text,
                        visualization_json: None,
                        usage: total_usage,
                    });
                }
                "tool_use" => {
                    let tool_uses: Vec<_> = self
                        .extract_tool_uses(&response)
                        .into_iter()
                        .cloned()
                        .collect();

                    if tool_uses.is_empty() {
                        let final_text = response
                            .content
                            .iter()
                            .filter_map(|block| match block {
                                ContentBlock::Text { text } => Some(text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        return Ok(LlmResponse {
                            messages: messages.clone(),
                            final_text,
                            visualization_json: None,
                            usage: total_usage,
                        });
                    }

                    let mut tool_results = Vec::new();
                    for tu in &tool_uses {
                        if let ContentBlock::ToolUse { id, .. } = tu {
                            let result = self.dispatch_tool(tu)?;
                            tool_results.push(ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: result,
                            });
                        }
                    }

                    messages.push(Message {
                        role: "user".into(),
                        content: tool_results,
                    });
                }
                _ => {
                    let final_text = response
                        .content
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    return Ok(LlmResponse {
                        messages: messages.clone(),
                        final_text,
                        visualization_json: None,
                        usage: total_usage,
                    });
                }
            }
        }

        Err(LlmError::MaxToolRounds(self.max_tool_rounds))
    }
}
