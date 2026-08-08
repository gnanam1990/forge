//! The agent loop: a provider produces assistant turns, the loop executes any
//! tool calls the assistant requests, feeds the results back, and repeats until
//! the assistant stops calling tools or the turn budget is exhausted.

pub mod http;
pub mod mock;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

use crate::context::{ContextManager, ProviderSummarizer};
use crate::error::{Error, Result};
use crate::hooks::{HookContext, HookDispatcher};
use crate::permission::{Approver, Permission, Policy};
use crate::tools::{Registry, ToolContext};

/// A tool call requested by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Message {
    /// The raw text payload of the message (used for token estimation).
    pub fn text(&self) -> String {
        match self {
            Message::System(t) | Message::User(t) => t.clone(),
            Message::Assistant { content, .. } => content.clone(),
            Message::Tool { output, .. } => output.clone(),
        }
    }

    /// A rough estimate of the message's token count (~4 chars per token).
    pub fn text_len(&self) -> usize {
        self.text().chars().count() / 4
    }
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

/// The agent: a provider plus a tool registry, driven by a turn budget, with a
/// permission policy and an optional approver.
pub struct Agent {
    provider: Arc<dyn Provider>,
    registry: Registry,
    max_turns: usize,
    policy: Policy,
    approver: Option<Box<Approver>>,
    context: ContextManager,
    workspace_root: PathBuf,
    hooks: HookDispatcher,
}

impl Agent {
    pub fn new(provider: Box<dyn Provider>, registry: Registry, max_turns: usize) -> Self {
        let provider = Arc::from(provider);
        let context = ContextManager::new(100_000)
            .with_summarizer(Arc::new(ProviderSummarizer::new(Arc::clone(&provider))));
        Self {
            provider,
            registry,
            max_turns,
            policy: Policy::new(),
            approver: None,
            context,
            workspace_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            hooks: HookDispatcher::new(),
        }
    }

    /// Set the permission policy.
    pub fn with_policy(mut self, policy: Policy) -> Self {
        self.policy = policy;
        self
    }

    /// Set an approver for prompt-level tools. Without one, prompt-level tools
    /// are denied in headless runs.
    pub fn with_approver(mut self, approver: Box<Approver>) -> Self {
        self.approver = Some(approver);
        self
    }

    /// Set the context manager (token budget + compaction).
    pub fn with_context(mut self, context: ContextManager) -> Self {
        self.context = context;
        self
    }

    /// Set the workspace root tools operate inside.
    pub fn with_workspace(mut self, root: impl Into<PathBuf>) -> Self {
        self.workspace_root = root.into();
        self
    }

    /// Set the hook dispatcher.
    pub fn with_hooks(mut self, hooks: HookDispatcher) -> Self {
        self.hooks = hooks;
        self
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
        let mut messages = vec![Message::System(self.system_prompt())];
        self.run_into(&mut messages, prompt)
    }

    /// Run the agent on a prompt, appending to an existing message list. This is
    /// the resume path: pass a session's messages and the conversation continues.
    pub fn run_into(&self, messages: &mut Vec<Message>, prompt: &str) -> Result<AgentOutcome> {
        messages.push(Message::User(prompt.to_string()));
        let mut tool_calls = 0usize;

        for turn in 0..self.max_turns {
            self.context.compact(messages);
            let assistant = self.provider.complete(messages)?;
            if assistant.tool_calls.is_empty() {
                messages.push(Message::Assistant {
                    content: assistant.content.clone(),
                    tool_calls: vec![],
                });
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

            let ctx = ToolContext::new(self.workspace_root.clone());
            for call in &assistant.tool_calls {
                tool_calls += 1;
                let output = self.execute_tool(call, &ctx);
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

    /// Execute a single tool call, applying the permission policy and the
    /// approver. Returns the text fed back to the model.
    fn execute_tool(&self, call: &ToolCall, ctx: &ToolContext) -> String {
        let Some(tool) = self.registry.get(&call.name) else {
            return format!("unknown tool: {}", call.name);
        };
        let effective = self.policy.decide(&call.name, tool.permission());
        let hook_ctx = HookContext {
            tool: call.name.clone(),
            args: call.arguments.clone(),
        };
        if let Err(e) = self.hooks.run_before(&hook_ctx) {
            return format!("blocked by hook: {e}");
        }
        let output = match effective {
            Permission::Deny => format!("permission denied: {}", call.name),
            Permission::Prompt => {
                let approved = self
                    .approver
                    .as_ref()
                    .map(|a| a(&call.name))
                    .unwrap_or(false);
                if approved {
                    self.run_tool(tool, call, ctx)
                } else {
                    format!("permission denied: {}", call.name)
                }
            }
            Permission::Allow => self.run_tool(tool, call, ctx),
        };
        self.hooks.run_after(&hook_ctx);
        output
    }

    fn run_tool(
        &self,
        tool: &dyn crate::tools::Tool,
        call: &ToolCall,
        ctx: &ToolContext,
    ) -> String {
        match tool.run(&call.arguments, ctx) {
            Ok(result) => result.output,
            Err(e) => format!("tool error: {e}"),
        }
    }

    /// Run a single prompt as an independent sub-agent sharing this agent's
    /// provider, returning the final text. Used by orchestration.
    pub fn run_prompt(&self, prompt: &str) -> Result<String> {
        let sub = Agent {
            provider: Arc::clone(&self.provider),
            registry: Registry::builtin(),
            max_turns: self.max_turns,
            policy: self.policy.clone(),
            approver: None,
            context: self.context.clone(),
            workspace_root: self.workspace_root.clone(),
            hooks: HookDispatcher::new(),
        };
        sub.run(prompt).map(|outcome| outcome.final_text)
    }

    /// Run several prompts as independent sub-agents in parallel, each with its
    /// own fresh context and tool registry, sharing this agent's provider. The
    /// results are returned in the same order as the prompts.
    pub fn run_parallel(&self, prompts: &[String]) -> Result<Vec<String>> {
        let policy = self.policy.clone();
        let handles: Vec<_> = prompts
            .iter()
            .map(|prompt| {
                let provider = Arc::clone(&self.provider);
                let prompt = prompt.clone();
                let max_turns = self.max_turns;
                let policy = policy.clone();
                let context = self.context.clone();
                let workspace_root = self.workspace_root.clone();
                std::thread::spawn(move || {
                    let sub = Agent {
                        provider,
                        registry: Registry::builtin(),
                        max_turns,
                        policy,
                        approver: None,
                        context,
                        workspace_root,
                        hooks: HookDispatcher::new(),
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
