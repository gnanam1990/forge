//! `git_log` — show recent commit history in the workspace.

use std::process::Command;

use serde_json::Value;

use super::{Tool, ToolContext, ToolResult};
use crate::error::Result;

#[derive(Default)]
pub struct GitLogTool;

impl GitLogTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitLogTool {
    fn name(&self) -> &str {
        "git_log"
    }

    fn description(&self) -> &str {
        "Show recent commit history in the workspace. Args: {\"limit\": number (optional, default 20)}."
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
        let limit_str = limit.to_string();
        let output = Command::new("git")
            .args(["log", "--oneline", "-n", &limit_str])
            .current_dir(&ctx.workspace_root)
            .output()?;
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            if text.trim().is_empty() {
                Ok(ToolResult::ok("no commits".to_string()))
            } else {
                Ok(ToolResult::ok(text))
            }
        } else {
            Ok(ToolResult::err(format!(
                "git log exited with {}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lists_commits_in_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(repo)
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "t@t.com"])
            .current_dir(repo)
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "tester"])
            .current_dir(repo)
            .status()
            .unwrap();
        std::fs::write(repo.join("a.txt"), "x").unwrap();
        Command::new("git")
            .args(["add", "a.txt"])
            .current_dir(repo)
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "-qm", "first"])
            .current_dir(repo)
            .status()
            .unwrap();

        let tool = GitLogTool::new();
        let ctx = ToolContext::new(repo.to_path_buf());
        let res = tool.run(&json!({ "limit": 5 }), &ctx).unwrap();
        assert!(res.ok);
        assert!(res.output.contains("first"));
    }

    #[test]
    fn reports_error_outside_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GitLogTool::new();
        let ctx = ToolContext::new(dir.path().to_path_buf());
        let res = tool.run(&json!({}), &ctx).unwrap();
        assert!(!res.ok);
    }
}
