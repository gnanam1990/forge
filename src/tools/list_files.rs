//! `list_files` — recursively list files (and directories) in the workspace.

use walkdir::WalkDir;

use serde_json::Value;

use super::{optional_string_arg, resolve_in_workspace, Tool, ToolContext, ToolResult};
use crate::error::Result;

#[derive(Default)]
pub struct ListFilesTool;

impl ListFilesTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "Recursively list files inside a directory of the workspace. Args: {\"path\": string (optional), \"limit\": number (optional, default 200)}."
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let raw = optional_string_arg(args, "path").unwrap_or_default();
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(200) as usize;
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
        let mut entries: Vec<String> = Vec::new();
        for entry in WalkDir::new(&resolved)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let rel = entry
                .path()
                .strip_prefix(&resolved)
                .unwrap_or(entry.path())
                .display()
                .to_string();
            let kind = if entry.file_type().is_dir() {
                "dir"
            } else {
                "file"
            };
            entries.push(format!("{kind}\t{rel}"));
            if entries.len() >= limit {
                break;
            }
        }
        entries.sort();
        if entries.is_empty() {
            return Ok(ToolResult::ok("(empty directory)"));
        }
        Ok(ToolResult::ok(entries.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lists_files_recursively() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("a.txt"), "x").unwrap();
        std::fs::write(dir.path().join("sub/b.txt"), "y").unwrap();
        let tool = ListFilesTool::new();
        let ctx = ToolContext::new(dir.path().to_path_buf());
        let res = tool.run(&json!({}), &ctx).unwrap();
        assert!(res.ok);
        assert!(res.output.contains("a.txt"));
        assert!(res.output.contains("sub/b.txt"));
    }

    #[test]
    fn honors_limit() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            std::fs::write(dir.path().join(format!("f{i}.txt")), "x").unwrap();
        }
        let tool = ListFilesTool::new();
        let ctx = ToolContext::new(dir.path().to_path_buf());
        let res = tool.run(&json!({ "limit": 2 }), &ctx).unwrap();
        assert!(res.ok);
        assert_eq!(res.output.lines().count(), 2);
    }
}
