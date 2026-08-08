//! `get_cwd` — return the current working directory.

use serde_json::Value;

use super::{Tool, ToolContext, ToolResult};
use crate::error::Result;

#[derive(Default)]
pub struct GetCwdTool;

impl GetCwdTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GetCwdTool {
    fn name(&self) -> &str {
        "get_cwd"
    }

    fn description(&self) -> &str {
        "Return the current working directory. Args: {}."
    }

    fn run(&self, _args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        Ok(ToolResult::ok(ctx.workspace_root.display().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn returns_workspace_root() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GetCwdTool::new();
        let ctx = ToolContext::new(dir.path().to_path_buf());
        let res = tool.run(&json!({}), &ctx).unwrap();
        assert!(res.ok);
        assert_eq!(res.output, dir.path().display().to_string());
    }
}
