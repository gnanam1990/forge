//! `write_file` — write a text file inside the workspace.

use std::fs;
use std::path::Path;

use serde_json::Value;

use super::{resolve_in_workspace, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

#[derive(Default)]
pub struct WriteFileTool;

impl WriteFileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write a text file inside the workspace, creating parent directories. Args: {\"path\": string, \"content\": string}."
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
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&resolved, content)?;
        Ok(ToolResult::ok(format!("wrote {}", resolved.display())))
    }
}

/// Helper used by tests and other tools to write a file inside a workspace.
pub fn write_workspace_file(root: &Path, rel: &str, content: &str) -> std::io::Result<PathBuf> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, content)?;
    Ok(path)
}

use std::path::PathBuf;
