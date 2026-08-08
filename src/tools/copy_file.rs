//! `copy_file` — copy a file inside the workspace.

use std::fs;

use serde_json::Value;

use super::{resolve_in_workspace, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

#[derive(Default)]
pub struct CopyFileTool;

impl CopyFileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for CopyFileTool {
    fn name(&self) -> &str {
        "copy_file"
    }

    fn description(&self) -> &str {
        "Copy a file to a new location inside the workspace. Args: {\"source\": string, \"dest\": string}."
    }

    fn permission(&self) -> Permission {
        Permission::Prompt
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let source = string_arg(args, "source")?;
        let dest = string_arg(args, "dest")?;
        let resolved = match resolve_in_workspace(&ctx.workspace_root, &source) {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::err(e.to_string())),
        };
        let target = match resolve_in_workspace(&ctx.workspace_root, &dest) {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::err(e.to_string())),
        };
        if !resolved.is_file() {
            return Ok(ToolResult::err(format!(
                "not a file: {}",
                resolved.display()
            )));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&resolved, &target)?;
        Ok(ToolResult::ok(format!(
            "copied {} to {}",
            resolved.display(),
            target.display()
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
    fn copies_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        fs::write(&src, "hello").unwrap();
        let tool = CopyFileTool::new();
        let res = tool
            .run(
                &json!({ "source": "a.txt", "dest": "sub/b.txt" }),
                &ctx(dir.path()),
            )
            .unwrap();
        assert!(res.ok);
        assert!(src.exists());
        assert_eq!(
            fs::read_to_string(dir.path().join("sub/b.txt")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn missing_source_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let tool = CopyFileTool::new();
        let res = tool
            .run(
                &json!({ "source": "nope.txt", "dest": "b.txt" }),
                &ctx(dir.path()),
            )
            .unwrap();
        assert!(!res.ok);
    }

    #[test]
    fn rejects_escape() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "x").unwrap();
        let tool = CopyFileTool::new();
        let res = tool
            .run(
                &json!({ "source": "a.txt", "dest": "../escape.txt" }),
                &ctx(dir.path()),
            )
            .unwrap();
        assert!(!res.ok);
    }
}
