//! `git_branch` — list branches in the workspace.

use std::process::Command;

use serde_json::Value;

use super::{Tool, ToolContext, ToolResult};
use crate::error::Result;

#[derive(Default)]
pub struct GitBranchTool;

impl GitBranchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitBranchTool {
    fn name(&self) -> &str {
        "git_branch"
    }

    fn description(&self) -> &str {
        "List the branches in the workspace (local and remote). Args: {\"all\": bool (optional, default true)}."
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let all = args.get("all").and_then(Value::as_bool).unwrap_or(true);
        let output = Command::new("git")
            .args(if all {
                vec!["branch", "-a"]
            } else {
                vec!["branch"]
            })
            .current_dir(&ctx.workspace_root)
            .output()?;
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            if text.trim().is_empty() {
                Ok(ToolResult::ok("no branches".to_string()))
            } else {
                Ok(ToolResult::ok(text))
            }
        } else {
            Ok(ToolResult::err(format!(
                "git branch exited with {}\n{}",
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
    fn lists_branches_in_a_repo() {
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

        let tool = GitBranchTool::new();
        let ctx = ToolContext::new(repo.to_path_buf());
        let res = tool.run(&json!({}), &ctx).unwrap();
        assert!(res.ok);
        assert!(res.output.contains("master") || res.output.contains("main"));
    }

    #[test]
    fn reports_error_outside_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GitBranchTool::new();
        let ctx = ToolContext::new(dir.path().to_path_buf());
        let res = tool.run(&json!({}), &ctx).unwrap();
        assert!(!res.ok);
    }
}
