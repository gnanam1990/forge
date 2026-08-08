//! `delete_file` — delete a file inside the workspace.

use std::fs;

use serde_json::Value;

use super::{resolve_in_workspace, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

#[derive(Default)]
pub struct DeleteFileTool;

impl DeleteFileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for DeleteFileTool {
    fn name(&self) -> &str {
        "delete_file"
    }

    fn description(&self) -> &str {
        "Delete a file inside the workspace. Args: {\"path\": string}."
    }

    fn permission(&self) -> Permission {
        Permission::Prompt
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path = string_arg(args, "path")?;
        let resolved = match resolve_in_workspace(&ctx.workspace_root, &path) {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::err(e.to_string())),
        };
        if resolved.is_dir() {
            return Ok(ToolResult::err(format!(
                "{} is a directory; delete_file only removes files",
                resolved.display()
            )));
        }
        if !resolved.exists() {
            return Ok(ToolResult::err(format!(
                "no such file: {}",
                resolved.display()
            )));
        }
        fs::remove_file(&resolved)?;
        Ok(ToolResult::ok(format!("deleted {}", resolved.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(root: &std::path::Path) -> ToolContext {
        ToolContext::new(root.to_path_buf())
    }

    #[test]
    fn deletes_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        fs::write(&path, "x").unwrap();
        let tool = DeleteFileTool::new();
        let res = tool
            .run(&json!({ "path": "a.txt" }), &ctx(dir.path()))
            .unwrap();
        assert!(res.ok);
        assert!(!path.exists());
    }

    #[test]
    fn missing_file_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let tool = DeleteFileTool::new();
        let res = tool
            .run(&json!({ "path": "nope.txt" }), &ctx(dir.path()))
            .unwrap();
        assert!(!res.ok);
    }

    #[test]
    fn refuses_directory() {
        let dir = tempfile::tempdir().unwrap();
        let tool = DeleteFileTool::new();
        let res = tool.run(&json!({ "path": "." }), &ctx(dir.path())).unwrap();
        assert!(!res.ok);
    }

    #[test]
    fn rejects_escape() {
        let dir = tempfile::tempdir().unwrap();
        let tool = DeleteFileTool::new();
        let res = tool
            .run(&json!({ "path": "../outside.txt" }), &ctx(dir.path()))
            .unwrap();
        assert!(!res.ok);
    }
}
