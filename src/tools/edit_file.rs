//! `edit_file` — replace text inside a file in the workspace.

use std::fs;

use serde_json::Value;

use super::{resolve_in_workspace, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;

#[derive(Default)]
pub struct EditFileTool;

impl EditFileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Replace the first occurrence of `old` with `new` in a file. Args: {\"path\": string, \"old\": string, \"new\": string}."
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path = string_arg(args, "path")?;
        let old = string_arg(args, "old")?;
        let new = string_arg(args, "new")?;
        let resolved = match resolve_in_workspace(&ctx.workspace_root, &path) {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::err(e.to_string())),
        };
        if !resolved.is_file() {
            return Ok(ToolResult::err(format!(
                "not a file: {}",
                resolved.display()
            )));
        }
        let content = fs::read_to_string(&resolved)?;
        if old.is_empty() {
            return Ok(ToolResult::err("`old` must not be empty"));
        }
        match content.find(&old) {
            Some(idx) => {
                let mut updated = content.clone();
                updated.replace_range(idx..idx + old.len(), &new);
                fs::write(&resolved, updated)?;
                Ok(ToolResult::ok(format!("edited {}", resolved.display())))
            }
            None => Ok(ToolResult::err(format!(
                "pattern not found in {}",
                resolved.display()
            ))),
        }
    }
}
