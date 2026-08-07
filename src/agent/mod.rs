//! The agent loop: a provider produces assistant turns, the loop executes any
//! tool calls the assistant requests, feeds the results back, and repeats until
//! the assistant stops calling tools or the turn budget is exhausted.

pub mod http;
pub mod mock;

use serde_json::Value;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::tools::{Registry, ToolContext};

/// A tool call requested by the assistant.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// A message in the conversation.
#[derive(Debug, Clone)]
pub enum Message {
    System(String),
    User(String),
    Assistant {
        content: String,
        tool_calls: Vec<ToolCall>,
    },
    Tool {
        tool_call_id: String,
        name: String,
        output: String,
    },
}

/// The assistant's reply to a turn.
#[derive(Debug, Clone)]
pub struct AssistantMessage {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

/// A model backend. Implementations translate the message list into a model
/// call and return the assistant's reply.
pub trait Provider: Send + Sync {
    fn complete(&self, messages: &[Message]) -> Result<AssistantMessage>;
}

/// The outcome of a completed agent run.
#[derive(Debug, Clone)]
pub struct AgentOutcome {
    pub final_text: String,
    pub turns: usize,
    pub tool_calls: usize,
}

/// The agent: a provider plus a tool registry, driven by a turn budget.
pub struct Agent {
    provider: Arc<dyn Provider>,
    registry: Registry,
    max_turns: usize,
}

impl Agent {
    pub fn new(provider: Box<dyn Provider>, registry: Registry, max_turns: usize) -> Self {
        Self {
            provider: Arc::from(provider),
            registry,
            max_turns,
        }
    }

    /// Build the system prompt that tells the model which tools are available.
    fn system_prompt(&self) -> String {
        let mut prompt = String::from(
            "You are forge, a coding agent. You work inside a workspace and use tools to \
             inspect and modify it. Available tools:\n",
        );
        for name in self.registry.names() {
            if let Some(tool) = self.registry.get(&name) {
                prompt.push_str(&format!("- {}: {}\n", tool.name(), tool.description()));
            }
        }
        prompt.push_str("\nCall tools by returning a JSON tool call. When done, answer plainly.");
        prompt
    }

    /// Run the agent on a user prompt until it stops calling tools or the turn
    /// budget is exhausted.
    pub fn run(&self, prompt: &str) -> Result<AgentOutcome> {
        let mut messages = vec![
            Message::System(self.system_prompt()),
            Message::User(prompt.to_string()),
        ];
        let mut tool_calls = 0usize;

        for turn in 0..self.max_turns {
            let assistant = self.provider.complete(&messages)?;
            if assistant.tool_calls.is_empty() {
                return Ok(AgentOutcome {
                    final_text: assistant.content,
                    turns: turn + 1,
                    tool_calls,
                });
            }

            messages.push(Message::Assistant {
                content: assistant.content,
                tool_calls: assistant.tool_calls.clone(),
            });

            let ctx = ToolContext::new(std::env::current_dir().unwrap_or_default());
            for call in &assistant.tool_calls {
                tool_calls += 1;
                let output = match self.registry.get(&call.name) {
                    Some(tool) => match tool.run(&call.arguments, &ctx) {
                        Ok(result) => result.output,
                        Err(e) => format!("tool error: {e}"),
                    },
                    None => format!("unknown tool: {}", call.name),
                };
                messages.push(Message::Tool {
                    tool_call_id: call.id.clone(),
                    name: call.name.clone(),
                    output,
                });
            }
        }

        Err(Error::Agent(format!(
            "reached max turns ({}) without a final answer",
            self.max_turns
        )))
    }

    /// Run several prompts as independent sub-agents in parallel, each with its
    /// own fresh context and tool registry, sharing this agent's provider. The
    /// results are returned in the same order as the prompts.
    pub fn run_parallel(&self, prompts: &[String]) -> Result<Vec<String>> {
        let handles: Vec<_> = prompts
            .iter()
            .map(|prompt| {
                let provider = Arc::clone(&self.provider);
                let prompt = prompt.clone();
                let max_turns = self.max_turns;
                std::thread::spawn(move || {
                    let sub = Agent {
                        provider,
                        registry: Registry::builtin(),
                        max_turns,
                    };
                    sub.run(&prompt).map(|outcome| outcome.final_text)
                })
            })
            .collect();

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            let joined = handle
                .join()
                .map_err(|_| Error::Agent("sub-agent panicked".into()))?;
            results.push(joined?);
        }
        Ok(results)
    }
}
