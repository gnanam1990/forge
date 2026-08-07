//! `terminal` — run a command in the workspace and return its output. A simple
//! virtual-terminal tool for interactive-style commands.

use std::process::Command;

use serde_json::Value;

use super::{string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

#[derive(Default)]
pub struct TerminalTool;

impl TerminalTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for TerminalTool {
    fn name(&self) -> &str {
        "terminal"
    }

    fn description(&self) -> &str {
        "Run a command in the workspace and return its output. Args: {\"command\": string}."
    }

    fn permission(&self) -> Permission {
        Permission::Prompt
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let command = string_arg(args, "command")?;
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .current_dir(&ctx.workspace_root)
            .output()?;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        if !output.stderr.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&String::from_utf8_lossy(&output.stderr));
        }
        if output.status.success() {
            Ok(ToolResult::ok(text))
        } else {
            Ok(ToolResult::err(format!(
                "command exited with {}\n{text}",
                output.status
            )))
        }
    }
}
