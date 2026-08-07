//! `apply_patch` — apply a unified diff to the workspace.

use std::process::Command;

use serde_json::Value;

use super::{string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

#[derive(Default)]
pub struct ApplyPatchTool;

impl ApplyPatchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Apply a unified diff to the workspace. Args: {\"patch\": string}."
    }

    fn permission(&self) -> Permission {
        Permission::Prompt
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let patch = string_arg(args, "patch")?;
        // Write the patch to a temp file and apply it with `patch -p1`.
        let patch_path = ctx.workspace_root.join(".forge-patch.tmp");
        std::fs::write(&patch_path, patch)?;
        let output = Command::new("patch")
            .args(["-p1", "-i"])
            .arg(&patch_path)
            .current_dir(&ctx.workspace_root)
            .output()?;
        let _ = std::fs::remove_file(&patch_path);
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        if !output.stderr.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&String::from_utf8_lossy(&output.stderr));
        }
        if output.status.success() {
            Ok(ToolResult::ok(text))
        } else {
            Ok(ToolResult::err(format!("patch failed: {text}")))
        }
    }
}
