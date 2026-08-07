//! `list_directory` — list the entries of a directory in the workspace.

use std::fs;

use serde_json::Value;

use super::{optional_string_arg, resolve_in_workspace, Tool, ToolContext, ToolResult};
use crate::error::Result;

#[derive(Default)]
pub struct ListDirectoryTool;

impl ListDirectoryTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ListDirectoryTool {
    fn name(&self) -> &str {
        "list_directory"
    }

    fn description(&self) -> &str {
        "List the entries of a directory inside the workspace. Args: {\"path\": string (optional, defaults to workspace root)}."
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let raw = optional_string_arg(args, "path").unwrap_or_default();
        let resolved = if raw.is_empty() {
            ctx.workspace_root.clone()
        } else {
            match resolve_in_workspace(&ctx.workspace_root, &raw) {
                Ok(p) => p,
                Err(e) => return Ok(ToolResult::err(e.to_string())),
            }
        };
        if !resolved.is_dir() {
            return Ok(ToolResult::err(format!(
                "not a directory: {}",
                resolved.display()
            )));
        }
        let mut entries: Vec<String> = fs::read_dir(&resolved)?
            .filter_map(|e| e.ok())
            .map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let kind = if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    "dir"
                } else {
                    "file"
                };
                format!("{kind}\t{name}")
            })
            .collect();
        entries.sort();
        if entries.is_empty() {
            return Ok(ToolResult::ok("(empty directory)"));
        }
        Ok(ToolResult::ok(entries.join("\n")))
    }
}
