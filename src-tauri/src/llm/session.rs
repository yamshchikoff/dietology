use std::fs;
use std::path::PathBuf;

use super::types::{ContentBlock, Message, Usage};

/// Состояние диалога: история сообщений, системный промпт, учёт токенов.
///
/// Не знает об HTTP, API, ToolRegistry. Чистый state holder.
pub struct ChatSession {
    pub messages: Vec<Message>,
    pub system_prompt: String,
    pub total_usage: Usage,
}

impl ChatSession {
    /// Новая сессия с заданным системным промптом.
    pub fn new(system_prompt: String) -> Self {
        Self {
            messages: Vec::new(),
            system_prompt,
            total_usage: Usage {
                input_tokens: 0,
                output_tokens: 0,
            },
        }
    }

    /// Добавить сообщение пользователя в историю.
    pub fn add_user_message(&mut self, text: String) {
        self.messages.push(Message {
            role: "user".into(),
            content: vec![ContentBlock::Text { text }],
        });
    }

    /// Аккумулировать usage после ответа модели.
    pub fn add_usage(&mut self, usage: Usage) {
        self.total_usage.input_tokens += usage.input_tokens;
        self.total_usage.output_tokens += usage.output_tokens;
    }

    /// Количество сообщений в истории.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Очистить историю (начать новый диалог с тем же системным промптом).
    pub fn clear(&mut self) {
        self.messages.clear();
        self.total_usage = Usage {
            input_tokens: 0,
            output_tokens: 0,
        };
    }

    // ---- JSONL сохранение/загрузка ----

    /// Сохранить историю в JSONL-файл.
    ///
    /// Формат: каждая строка — JSON-объект Message.
    /// Системный промпт сохраняется как первое сообщение с role="system".
    pub fn save_to_jsonl(&self, path: &PathBuf) -> Result<(), String> {
        let mut lines = Vec::new();

        // Системный промпт как первая "system" запись
        lines.push(
            serde_json::to_string(&serde_json::json!({
                "role": "system",
                "content": self.system_prompt,
            }))
            .map_err(|e| format!("failed to serialize system prompt: {e}"))?,
        );

        for msg in &self.messages {
            let line = serde_json::to_string(msg)
                .map_err(|e| format!("failed to serialize message: {e}"))?;
            lines.push(line);
        }

        let content = lines.join("\n") + "\n";
        fs::write(path, content).map_err(|e| format!("failed to write {path:?}: {e}"))
    }

    /// Загрузить историю из JSONL-файла.
    ///
    /// Первая строка с role="system" становится system_prompt.
    /// Остальные строки парсятся как Message.
    pub fn load_from_jsonl(path: &PathBuf) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("failed to read {path:?}: {e}"))?;

        let mut messages = Vec::new();
        let mut system_prompt = String::new();

        for (i, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let value: serde_json::Value = serde_json::from_str(line)
                .map_err(|e| format!("line {i}: invalid JSON: {e}"))?;

            let role = value["role"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            if role == "system" {
                system_prompt = value["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
            } else {
                let msg: Message = serde_json::from_value(value)
                    .map_err(|e| format!("line {i}: failed to parse Message: {e}"))?;
                messages.push(msg);
            }
        }

        Ok(Self {
            messages,
            system_prompt,
            total_usage: Usage {
                input_tokens: 0, // usage не сохраняется — только текущая сессия
                output_tokens: 0,
            },
        })
    }
}
