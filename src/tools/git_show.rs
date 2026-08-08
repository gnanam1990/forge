//! `git_show` — show a commit or a file's content at a revision.

use std::process::Command;

use serde_json::Value;

use super::{string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;

#[derive(Default)]
pub struct GitShowTool;

impl GitShowTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitShowTool {
    fn name(&self) -> &str {
        "git_show"
    }

    fn description(&self) -> &str {
        "Show a commit, or a file's content at a revision. Args: {\"reference\": string} (e.g. \"HEAD\", \"HEAD:src/main.rs\")."
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let reference = string_arg(args, "reference")?;
        let output = Command::new("git")
            .args(["show", &reference])
            .current_dir(&ctx.workspace_root)
            .output()?;
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            if text.trim().is_empty() {
                Ok(ToolResult::ok(format!("no output for {reference}")))
            } else {
                Ok(ToolResult::ok(text))
            }
        } else {
            Ok(ToolResult::err(format!(
                "git show {reference} exited with {}\n{}",
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
    fn shows_a_commit() {
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
        std::fs::write(repo.join("a.txt"), "hello").unwrap();
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

        let tool = GitShowTool::new();
        let ctx = ToolContext::new(repo.to_path_buf());
        let res = tool.run(&json!({ "reference": "HEAD" }), &ctx).unwrap();
        assert!(res.ok);
        assert!(res.output.contains("first"));
    }

    #[test]
    fn shows_a_file_at_a_revision() {
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
        std::fs::write(repo.join("a.txt"), "hello").unwrap();
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

        let tool = GitShowTool::new();
        let ctx = ToolContext::new(repo.to_path_buf());
        let res = tool
            .run(&json!({ "reference": "HEAD:a.txt" }), &ctx)
            .unwrap();
        assert!(res.ok);
        assert!(res.output.contains("hello"));
    }

    #[test]
    fn reports_error_outside_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GitShowTool::new();
        let ctx = ToolContext::new(dir.path().to_path_buf());
        let res = tool.run(&json!({ "reference": "HEAD" }), &ctx).unwrap();
        assert!(!res.ok);
    }
}
