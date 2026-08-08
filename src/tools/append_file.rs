//! `append_file` — append text to a file inside the workspace.

use std::fs;
use std::io::Write;

use serde_json::Value;

use super::{resolve_in_workspace, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

#[derive(Default)]
pub struct AppendFileTool;

impl AppendFileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for AppendFileTool {
    fn name(&self) -> &str {
        "append_file"
    }

    fn description(&self) -> &str {
        "Append text to the end of a file inside the workspace, creating it if missing. Args: {\"path\": string, \"content\": string}."
    }

    fn permission(&self) -> Permission {
        Permission::Prompt
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path = string_arg(args, "path")?;
        let content = string_arg(args, "content")?;
        let resolved = match resolve_in_workspace(&ctx.workspace_root, &path) {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::err(e.to_string())),
        };
        if resolved.is_dir() {
            return Ok(ToolResult::err(format!(
                "{} is a directory; append_file only writes files",
                resolved.display()
            )));
        }
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&resolved)?;
        file.write_all(content.as_bytes())?;
        file.write_all(b"\n")?;
        Ok(ToolResult::ok(format!(
            "appended to {}",
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
    fn appends_to_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.txt");
        fs::write(&path, "line1").unwrap();
        let tool = AppendFileTool::new();
        let res = tool
            .run(
                &json!({ "path": "log.txt", "content": "line2" }),
                &ctx(dir.path()),
            )
            .unwrap();
        assert!(res.ok);
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("line1"));
        assert!(raw.contains("line2"));
    }

    #[test]
    fn creates_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let tool = AppendFileTool::new();
        let res = tool
            .run(
                &json!({ "path": "new.txt", "content": "hi" }),
                &ctx(dir.path()),
            )
            .unwrap();
        assert!(res.ok);
        assert!(dir.path().join("new.txt").exists());
    }

    #[test]
    fn rejects_escape() {
        let dir = tempfile::tempdir().unwrap();
        let tool = AppendFileTool::new();
        let res = tool
            .run(
                &json!({ "path": "../escape.txt", "content": "x" }),
                &ctx(dir.path()),
            )
            .unwrap();
        assert!(!res.ok);
    }
}
