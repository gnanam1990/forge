//! `make_directory` — create a directory (and parents) inside the workspace.

use std::fs;

use serde_json::Value;

use super::{resolve_in_workspace, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

#[derive(Default)]
pub struct MakeDirectoryTool;

impl MakeDirectoryTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for MakeDirectoryTool {
    fn name(&self) -> &str {
        "make_directory"
    }

    fn description(&self) -> &str {
        "Create a directory inside the workspace, including any parents. Args: {\"path\": string}."
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
        fs::create_dir_all(&resolved)?;
        Ok(ToolResult::ok(format!(
            "created directory {}",
            resolved.display()
        )))
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
    fn creates_directory_and_parents() {
        let dir = tempfile::tempdir().unwrap();
        let tool = MakeDirectoryTool::new();
        let res = tool
            .run(&json!({ "path": "a/b/c" }), &ctx(dir.path()))
            .unwrap();
        assert!(res.ok);
        assert!(dir.path().join("a/b/c").is_dir());
    }

    #[test]
    fn rejects_escape() {
        let dir = tempfile::tempdir().unwrap();
        let tool = MakeDirectoryTool::new();
        let res = tool
            .run(&json!({ "path": "../outside" }), &ctx(dir.path()))
            .unwrap();
        assert!(!res.ok);
    }
}
