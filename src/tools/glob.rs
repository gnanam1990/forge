//! `glob` — find files inside the workspace by a glob pattern.

use std::path::Path;

use serde_json::Value;
use walkdir::WalkDir;

use super::{optional_string_arg, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;

/// Convert a glob pattern to a regex, supporting `*`, `**`, and `?`.
fn glob_to_regex(pattern: &str) -> Result<regex::Regex> {
    let mut out = String::from("^");
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    i += 1;
                    if i + 1 < chars.len() && chars[i + 1] == '/' {
                        i += 1;
                        out.push_str("(?:.*/)?");
                    } else {
                        out.push_str(".*");
                    }
                } else {
                    out.push_str("[^/]*");
                }
            }
            '?' => out.push_str("[^/]"),
            c => out.push_str(&regex::escape(&c.to_string())),
        }
        i += 1;
    }
    out.push('$');
    regex::Regex::new(&out).map_err(|e| crate::error::Error::InvalidArgs(e.to_string()))
}

#[derive(Default)]
pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files inside the workspace by glob pattern. Args: {\"pattern\": string, \"limit\": number (optional)}."
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let pattern = string_arg(args, "pattern")?;
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(100) as usize;
        let base = optional_string_arg(args, "base").unwrap_or_default();
        let re = glob_to_regex(&pattern)?;

        let root = ctx.workspace_root.clone();
        let search_dir = if base.is_empty() {
            root.clone()
        } else {
            root.join(&base)
        };

        let mut matches = Vec::new();
        for entry in WalkDir::new(&search_dir)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(&root).unwrap_or(entry.path());
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if re.is_match(&rel_str) {
                matches.push(rel_str);
                if matches.len() >= limit {
                    break;
                }
            }
        }
        matches.sort();
        if matches.is_empty() {
            return Ok(ToolResult::ok("no matches"));
        }
        Ok(ToolResult::ok(matches.join("\n")))
    }
}

/// Helper to normalize a path for glob matching.
#[allow(dead_code)]
fn slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_patterns() {
        assert!(glob_to_regex("**/*.rs").unwrap().is_match("src/main.rs"));
        assert!(glob_to_regex("*.rs").unwrap().is_match("main.rs"));
        assert!(!glob_to_regex("*.rs").unwrap().is_match("src/main.rs"));
        assert!(glob_to_regex("a?c").unwrap().is_match("abc"));
    }
}
