//! `move_file` — move or rename a file inside the workspace.

use std::fs;

use serde_json::Value;

use super::{resolve_in_workspace, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

#[derive(Default)]
pub struct MoveFileTool;

impl MoveFileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for MoveFileTool {
    fn name(&self) -> &str {
        "move_file"
    }

    fn description(&self) -> &str {
        "Move or rename a file inside the workspace. Args: {\"source\": string, \"dest\": string}."
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
        if !resolved.exists() {
            return Ok(ToolResult::err(format!(
                "source does not exist: {}",
                resolved.display()
            )));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&resolved, &target)?;
        Ok(ToolResult::ok(format!(
            "moved {} to {}",
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
    fn moves_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        fs::write(&src, "hello").unwrap();
        let tool = MoveFileTool::new();
        let res = tool
            .run(
                &json!({ "source": "a.txt", "dest": "sub/b.txt" }),
                &ctx(dir.path()),
            )
            .unwrap();
        assert!(res.ok);
        assert!(!src.exists());
        assert!(dir.path().join("sub/b.txt").exists());
    }

    #[test]
    fn rejects_escape() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "x").unwrap();
        let tool = MoveFileTool::new();
        let res = tool
            .run(
                &json!({ "source": "a.txt", "dest": "../escape.txt" }),
                &ctx(dir.path()),
            )
            .unwrap();
        assert!(!res.ok);
    }

    #[test]
    fn missing_source_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let tool = MoveFileTool::new();
        let res = tool
            .run(
                &json!({ "source": "nope.txt", "dest": "b.txt" }),
                &ctx(dir.path()),
            )
            .unwrap();
        assert!(!res.ok);
    }
}
