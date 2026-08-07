//! `grep` — search file contents inside the workspace by regex.

use serde_json::Value;
use walkdir::WalkDir;

use super::{optional_string_arg, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;

#[derive(Default)]
pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search file contents inside the workspace by regex. Args: {\"pattern\": string, \"glob\": string (optional), \"limit\": number (optional)}."
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let pattern = string_arg(args, "pattern")?;
        let glob = optional_string_arg(args, "glob").unwrap_or_default();
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(100) as usize;
        let re = regex::Regex::new(&pattern)
            .map_err(|e| crate::error::Error::InvalidArgs(e.to_string()))?;

        let root = ctx.workspace_root.clone();
        let mut hits = Vec::new();
        for entry in WalkDir::new(&root)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(&root).unwrap_or(entry.path());
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if !glob.is_empty() && !rel_str.contains(&glob) {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            for (idx, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    hits.push(format!("{}:{}:{}", rel_str, idx + 1, line));
                    if hits.len() >= limit {
                        break;
                    }
                }
            }
            if hits.len() >= limit {
                break;
            }
        }
        if hits.is_empty() {
            return Ok(ToolResult::ok("no matches"));
        }
        Ok(ToolResult::ok(hits.join("\n")))
    }
}
