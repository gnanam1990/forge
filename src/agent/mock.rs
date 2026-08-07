//! A scriptable mock provider for tests and local demos. It replays a fixed
//! script of assistant turns, so the agent loop can be exercised without a model.

use std::sync::Mutex;

use serde_json::json;

use super::{AssistantMessage, Message, Provider, ToolCall};
use crate::error::Result;

/// A scripted turn: either a final answer or a set of tool calls.
#[derive(Debug, Clone)]
pub enum ScriptTurn {
    Answer(String),
    Tools(Vec<ToolCall>),
}

/// A provider that replays a script, then answers. Used to test the loop.
pub struct MockProvider {
    script: Mutex<Vec<ScriptTurn>>,
}

impl MockProvider {
    pub fn new(script: Vec<ScriptTurn>) -> Self {
        Self {
            script: Mutex::new(script),
        }
    }

    /// A convenience builder for a single tool call followed by an answer.
    pub fn tool_then_answer(tool: &str, args: serde_json::Value, answer: &str) -> Self {
        Self::new(vec![
            ScriptTurn::Tools(vec![ToolCall {
                id: "call_1".into(),
                name: tool.into(),
                arguments: args,
            }]),
            ScriptTurn::Answer(answer.into()),
        ])
    }
}

impl Provider for MockProvider {
    fn complete(&self, _messages: &[Message]) -> Result<AssistantMessage> {
        let mut script = self.script.lock().unwrap();
        if script.is_empty() {
            return Ok(AssistantMessage {
                content: "done".into(),
                tool_calls: vec![],
            });
        }
        match script.remove(0) {
            ScriptTurn::Answer(content) => Ok(AssistantMessage {
                content,
                tool_calls: vec![],
            }),
            ScriptTurn::Tools(calls) => Ok(AssistantMessage {
                content: String::new(),
                tool_calls: calls,
            }),
        }
    }
}

/// A helper to build a tool call quickly in tests.
pub fn call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: name.into(),
        arguments: args,
    }
}

/// A helper to build a read_file call.
pub fn read_call(id: &str, path: &str) -> ToolCall {
    call(id, "read_file", json!({ "path": path }))
}
