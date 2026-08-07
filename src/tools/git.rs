//! Git integration tools: status, diff, and commit.

use std::process::Command;

use serde_json::Value;

use super::{string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

/// Run a git command in the workspace and return its combined output.
fn git(ctx: &ToolContext, args: &[&str]) -> Result<ToolResult> {
    let output = Command::new("git")
        .args(args)
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
            "git exited with {}\n{text}",
            output.status
        )))
    }
}

#[derive(Default)]
pub struct GitStatusTool;

impl GitStatusTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }

    fn description(&self) -> &str {
        "Show the working tree status. Args: {}."
    }

    fn run(&self, _args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        git(ctx, &["status", "--short"])
    }
}

#[derive(Default)]
pub struct GitDiffTool;

impl GitDiffTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitDiffTool {
    fn name(&self) -> &str {
        "git_diff"
    }

    fn description(&self) -> &str {
        "Show uncommitted changes. Args: {}."
    }

    fn run(&self, _args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        git(ctx, &["diff"])
    }
}

#[derive(Default)]
pub struct GitCommitTool;

impl GitCommitTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitCommitTool {
    fn name(&self) -> &str {
        "git_commit"
    }

    fn description(&self) -> &str {
        "Stage all changes and commit with a message. Args: {\"message\": string}."
    }

    fn permission(&self) -> Permission {
        Permission::Prompt
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let message = string_arg(args, "message")?;
        git(ctx, &["add", "-A"])?;
        git(ctx, &["commit", "-m", &message])
    }
}
