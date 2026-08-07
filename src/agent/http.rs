//! An OpenAI-compatible HTTP provider. It posts the message list to a chat
//! completions endpoint and parses the assistant reply, including any tool
//! calls the model requests.

use serde_json::{json, Value};

use super::{AssistantMessage, Message, Provider, ToolCall};
use crate::config::ProviderConfig;
use crate::error::{Error, Result};

/// A provider backed by any OpenAI-compatible chat completions API.
pub struct HttpProvider {
    base_url: String,
    model: String,
    api_key: String,
    client: reqwest::blocking::Client,
}

impl HttpProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self> {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let model = config
            .model
            .clone()
            .ok_or_else(|| Error::Provider("no model configured".into()))?;
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("FORGE_API_KEY").ok())
            .ok_or_else(|| {
                Error::Provider("no API key; set provider.api_key or FORGE_API_KEY".into())
            })?;
        let client = reqwest::blocking::Client::new();
        Ok(Self {
            base_url,
            model,
            api_key,
            client,
        })
    }

    fn to_api_messages(messages: &[Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|m| match m {
                Message::System(text) => json!({ "role": "system", "content": text }),
                Message::User(text) => json!({ "role": "user", "content": text }),
                Message::Assistant {
                    content,
                    tool_calls,
                } => {
                    let calls: Vec<Value> = tool_calls
                        .iter()
                        .map(|c| {
                            json!({
                                "id": c.id,
                                "type": "function",
                                "function": { "name": c.name, "arguments": c.arguments.to_string() }
                            })
                        })
                        .collect();
                    json!({ "role": "assistant", "content": content, "tool_calls": calls })
                }
                Message::Tool {
                    tool_call_id,
                    name,
                    output,
                } => json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "name": name,
                    "content": output
                }),
            })
            .collect()
    }
}

impl Provider for HttpProvider {
    fn complete(&self, messages: &[Message]) -> Result<AssistantMessage> {
        let body = json!({
            "model": self.model,
            "messages": Self::to_api_messages(messages),
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "read_file",
                        "description": "Read a text file inside the workspace.",
                        "parameters": { "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "write_file",
                        "description": "Write a text file inside the workspace.",
                        "parameters": { "type": "object", "properties": { "path": { "type": "string" }, "content": { "type": "string" } }, "required": ["path", "content"] }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "bash",
                        "description": "Run a shell command.",
                        "parameters": { "type": "object", "properties": { "command": { "type": "string" } }, "required": ["command"] }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "glob",
                        "description": "Find files by glob pattern.",
                        "parameters": { "type": "object", "properties": { "pattern": { "type": "string" } }, "required": ["pattern"] }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "grep",
                        "description": "Search file contents by regex.",
                        "parameters": { "type": "object", "properties": { "pattern": { "type": "string" } }, "required": ["pattern"] }
                    }
                }
            ]
        });

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .map_err(|e| Error::Provider(format!("request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            return Err(Error::Provider(format!("HTTP {status}: {text}")));
        }

        let payload: Value = response
            .json()
            .map_err(|e| Error::Provider(e.to_string()))?;
        let choice = payload
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or_else(|| Error::Provider("no choices in response".into()))?;
        let message = choice
            .get("message")
            .ok_or_else(|| Error::Provider("no message".into()))?;
        let content = message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let mut tool_calls = Vec::new();
        if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
            for c in calls {
                let id = c
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let name = c
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let arguments = c
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(Value::as_str)
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(Value::Null);
                tool_calls.push(ToolCall {
                    id,
                    name,
                    arguments,
                });
            }
        }

        Ok(AssistantMessage {
            content,
            tool_calls,
        })
    }
}
