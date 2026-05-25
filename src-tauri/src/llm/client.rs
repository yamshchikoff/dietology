use std::sync::Arc;

use crate::tools::registry::{ToolCall, ToolRegistry};

use futures::StreamExt;

use super::types::{
    ApiRequest, ApiResponse, ContentBlock, LlmError, LlmResponse, Message, SseContentBlock,
    SseDelta, SseMessage, StreamedResponse, Usage,
};

fn extract_text(blocks: &[ContentBlock]) -> String {
    blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn extract_tool_uses(blocks: &[ContentBlock]) -> Vec<&ContentBlock> {
    blocks
        .iter()
        .filter(|block| matches!(block, ContentBlock::ToolUse { .. }))
        .collect()
}

pub struct LlmClient {
    pub api_base_url: String,
    pub api_key: String,
    pub model: String,
    pub http: reqwest::Client,
    pub http_stream: reqwest::Client,
    pub registry: Arc<ToolRegistry>,
    pub max_tokens: u32,
}

/// Intermediate state for building a ContentBlock from SSE deltas.
enum BlockBuilder {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input_json: String,
    },
}

impl BlockBuilder {
    fn append_text(&mut self, delta: &str) {
        if let BlockBuilder::Text { ref mut text } = self {
            text.push_str(delta);
        }
    }

    fn append_json(&mut self, delta: &str) {
        if let BlockBuilder::ToolUse {
            ref mut input_json, ..
        } = self
        {
            input_json.push_str(delta);
        }
    }

    fn into_content_block(self) -> Result<ContentBlock, LlmError> {
        match self {
            BlockBuilder::Text { text } => Ok(ContentBlock::Text { text }),
            BlockBuilder::ToolUse {
                id,
                name,
                input_json,
            } => {
                let input: serde_json::Value = serde_json::from_str(&input_json)
                    .map_err(|e| LlmError::Parse(format!("tool input JSON: {e}")))?;
                Ok(ContentBlock::ToolUse { id, name, input })
            }
        }
    }
}

impl LlmClient {
    pub fn new(registry: Arc<ToolRegistry>) -> Result<Self, LlmError> {
        let api_key =
            std::env::var("DEEPSEEK_API_KEY").map_err(|_| LlmError::MissingApiKey)?;
        let api_base_url = std::env::var("DEEPSEEK_API_BASE")
            .unwrap_or_else(|_| "https://api.deepseek.com".into());
        let model =
            std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-chat".into());

        Self::with_credentials(registry, api_key, api_base_url, model)
    }

    pub fn with_credentials(
        registry: Arc<ToolRegistry>,
        api_key: String,
        api_base_url: String,
        model: String,
    ) -> Result<Self, LlmError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| LlmError::Network(e.to_string()))?;

        let http_stream = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| LlmError::Network(e.to_string()))?;

        Ok(Self {
            api_base_url,
            api_key,
            model,
            http,
            http_stream,
            registry,
            max_tokens: 4096,
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
            stream: false,
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

        loop {
            let response = self.call_api(messages, system_prompt).await?;

            total_usage.input_tokens += response.usage.input_tokens;
            total_usage.output_tokens += response.usage.output_tokens;

            let stop_reason = response.stop_reason;
            messages.push(Message {
                role: "assistant".into(),
                content: response.content,
            });

            match stop_reason.as_str() {
                "end_turn" | "max_tokens" => {
                    let final_text = extract_text(&messages.last().unwrap().content);
                    if final_text.is_empty() {
                        return Err(LlmError::Parse("no text in response".into()));
                    }
                    return Ok(LlmResponse {
                        final_text,
                        visualization_json: None,
                        usage: total_usage,
                    });
                }
                "tool_use" => {
                    let tool_results = {
                        let tool_uses =
                            extract_tool_uses(&messages.last().unwrap().content);

                        if tool_uses.is_empty() {
                            let final_text =
                                extract_text(&messages.last().unwrap().content);
                            if final_text.is_empty() {
                                return Err(LlmError::Parse(
                                    "no text in response".into(),
                                ));
                            }
                            return Ok(LlmResponse {
                                final_text,
                                visualization_json: None,
                                usage: total_usage,
                            });
                        }

                        let mut results = Vec::new();
                        for tu in tool_uses {
                            if let ContentBlock::ToolUse { id, .. } = tu {
                                let result = self.dispatch_tool(tu)?;
                                results.push(ContentBlock::ToolResult {
                                    tool_use_id: id.clone(),
                                    content: result,
                                });
                            }
                        }
                        results
                    };

                    messages.push(Message {
                        role: "user".into(),
                        content: tool_results,
                    });
                }
                other => {
                    return Err(LlmError::Parse(format!(
                        "unexpected stop_reason: {other}"
                    )));
                }
            }
        }
    }

    /// POST to the API with `stream: true`, parse SSE, fire callbacks for text/tool events.
    pub(crate) async fn call_api_with_stream<F1, F2>(
        &self,
        messages: &[Message],
        system: &str,
        mut on_token: F1,
        mut on_tool_start: F2,
    ) -> Result<StreamedResponse, LlmError>
    where
        F1: FnMut(&str),
        F2: FnMut(&str),
    {
        let url = format!("{}/v1/messages", self.api_base_url);
        let tools = self.registry.definitions();

        let request = ApiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            stream: true,
            system: system.to_string(),
            messages: messages.to_vec(),
            tools,
        };

        let response = self
            .http_stream
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
            let body = response.text().await.unwrap_or_else(|_| "unknown error".into());
            return Err(LlmError::Api {
                status: status.as_u16(),
                body,
            });
        }

        self.parse_sse_stream(response, &mut on_token, &mut on_tool_start)
            .await
    }

    async fn parse_sse_stream<F1, F2>(
        &self,
        response: reqwest::Response,
        on_token: &mut F1,
        on_tool_start: &mut F2,
    ) -> Result<StreamedResponse, LlmError>
    where
        F1: FnMut(&str),
        F2: FnMut(&str),
    {
        let mut stream = response.bytes_stream();

        let mut blocks: Vec<ContentBlock> = Vec::new();
        let mut current_block: Option<BlockBuilder> = None;
        let mut stop_reason: Option<String> = None;
        let mut usage: Option<Usage> = None;
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let bytes = chunk_result.map_err(|e| LlmError::Network(e.to_string()))?;
            let chunk_str = String::from_utf8_lossy(&bytes);
            buffer.push_str(&chunk_str);

            while let Some(event_end) = buffer.find("\n\n") {
                let event_text = buffer[..event_end].to_string();
                buffer = buffer[event_end + 2..].to_string();

                if event_text.trim().is_empty() {
                    continue;
                }

                let data_lines: Vec<&str> = event_text
                    .lines()
                    .filter(|line| line.starts_with("data: "))
                    .map(|line| &line[6..])
                    .collect();

                if data_lines.is_empty() {
                    continue;
                }

                let data_json = data_lines.join("");
                let sse_msg: SseMessage = serde_json::from_str(&data_json)
                    .map_err(|e| LlmError::Parse(format!("SSE parse error: {e}")))?;

                match sse_msg {
                    SseMessage::ContentBlockStart {
                        content_block, ..
                    } => match content_block {
                        SseContentBlock::Text { text } => {
                            if !text.is_empty() {
                                on_token(&text);
                            }
                            current_block = Some(BlockBuilder::Text { text });
                        }
                        SseContentBlock::ToolUse { id, name, .. } => {
                            on_tool_start(&name);
                            current_block = Some(BlockBuilder::ToolUse {
                                id,
                                name,
                                input_json: String::new(),
                            });
                        }
                        SseContentBlock::Other => {}
                    },
                    SseMessage::ContentBlockDelta { delta, .. } => match delta {
                        SseDelta::Text { text } => {
                            on_token(&text);
                            if let Some(ref mut builder) = current_block {
                                builder.append_text(&text);
                            }
                        }
                        SseDelta::InputJson { partial_json } => {
                            if let Some(ref mut builder) = current_block {
                                builder.append_json(&partial_json);
                            }
                        }
                        SseDelta::Other => {}
                    },
                    SseMessage::ContentBlockStop { .. } => {
                        if let Some(builder) = current_block.take() {
                            blocks.push(builder.into_content_block()?);
                        }
                    }
                    SseMessage::MessageDelta { delta, usage: u } => {
                        stop_reason = Some(delta.stop_reason);
                        usage = u;
                    }
                    SseMessage::MessageStop
                    | SseMessage::Ping
                    | SseMessage::MessageStart
                    | SseMessage::Unknown => {}
                }
            }
        }

        let stop_reason = stop_reason.unwrap_or_else(|| "end_turn".into());
        let usage = usage.unwrap_or(Usage {
            input_tokens: 0,
            output_tokens: 0,
        });

        Ok(StreamedResponse {
            content: blocks,
            stop_reason,
            usage,
        })
    }

    /// Multi-round chat using SSE streaming for every API call.
    /// Fires text deltas and tool notifications via callbacks.
    pub async fn chat_streaming<F1, F2, F3>(
        &self,
        messages: &mut Vec<Message>,
        system_prompt: &str,
        mut on_token: F1,
        mut on_tool_start: F2,
        mut on_tool_done: F3,
    ) -> Result<LlmResponse, LlmError>
    where
        F1: FnMut(&str),
        F2: FnMut(&str),
        F3: FnMut(&str),
    {
        let mut total_usage = Usage {
            input_tokens: 0,
            output_tokens: 0,
        };

        loop {
            let response = self
                .call_api_with_stream(
                    messages,
                    system_prompt,
                    &mut on_token,
                    &mut on_tool_start,
                )
                .await?;

            total_usage.input_tokens += response.usage.input_tokens;
            total_usage.output_tokens += response.usage.output_tokens;

            let stop_reason = response.stop_reason;
            messages.push(Message {
                role: "assistant".into(),
                content: response.content,
            });

            match stop_reason.as_str() {
                "end_turn" | "max_tokens" => {
                    let final_text = extract_text(&messages.last().unwrap().content);
                    if final_text.is_empty() {
                        return Err(LlmError::Parse("no text in response".into()));
                    }
                    return Ok(LlmResponse {
                        final_text,
                        visualization_json: None,
                        usage: total_usage,
                    });
                }
                "tool_use" => {
                    let tool_uses =
                        extract_tool_uses(&messages.last().unwrap().content);

                    if tool_uses.is_empty() {
                        let final_text =
                            extract_text(&messages.last().unwrap().content);
                        if final_text.is_empty() {
                            return Err(LlmError::Parse("no text in response".into()));
                        }
                        return Ok(LlmResponse {
                            final_text,
                            visualization_json: None,
                            usage: total_usage,
                        });
                    }

                    let mut results = Vec::new();
                    for tu in tool_uses {
                        if let ContentBlock::ToolUse { id, name, .. } = tu {
                            let result = self.dispatch_tool(tu)?;
                            results.push(ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: result,
                            });
                            on_tool_done(name);
                        }
                    }

                    messages.push(Message {
                        role: "user".into(),
                        content: results,
                    });
                }
                other => {
                    return Err(LlmError::Parse(format!(
                        "unexpected stop_reason: {other}"
                    )));
                }
            }
        }

    }
}
