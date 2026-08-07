//! `ask_user` — ask the user a question and return their answer.

use std::io::Write;

use serde_json::Value;

use super::{string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;

/// A responder resolves a question to an answer. The default reads from stdin;
/// tests inject a fixed responder.
pub type Responder = dyn Fn(&str) -> String + Send + Sync;

#[derive(Default)]
pub struct AskUserTool {
    responder: Option<Box<Responder>>,
}

impl AskUserTool {
    pub fn new() -> Self {
        Self { responder: None }
    }

    /// Build a tool with a fixed responder (used in tests and headless runs).
    pub fn with_responder(responder: Box<Responder>) -> Self {
        Self {
            responder: Some(responder),
        }
    }
}

impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn description(&self) -> &str {
        "Ask the user a question and return their answer. Args: {\"question\": string}."
    }

    fn run(&self, args: &Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let question = string_arg(args, "question")?;
        let answer = match &self.responder {
            Some(responder) => responder(&question),
            None => {
                print!("{question} ");
                std::io::stdout().flush()?;
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                line.trim().to_string()
            }
        };
        Ok(ToolResult::ok(answer))
    }
}
