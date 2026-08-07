//! `read_file` — read a text file inside the workspace.

use std::fs;

use serde_json::Value;

use super::{resolve_in_workspace, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;

/// Cap on how much of a file is returned, to keep a single tool result bounded.
const MAX_BYTES: usize = 256 * 1024;

#[derive(Default)]
pub struct ReadFileTool;

impl ReadFileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read a text file inside the workspace. Args: {\"path\": string}."
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let path = string_arg(args, "path")?;
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
        let bytes = fs::read(&resolved)?;
        let truncated = bytes.len() > MAX_BYTES;
        let text = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_BYTES)]).into_owned();
        let mut output = text;
        if truncated {
            output.push_str(&format!("\n[truncated: file exceeds {} bytes]", MAX_BYTES));
        }
        Ok(ToolResult::ok(output))
    }
}
