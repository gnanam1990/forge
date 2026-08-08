//! `get_env` — read environment variables.

use serde_json::Value;

use super::{optional_string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;

#[derive(Default)]
pub struct GetEnvTool;

impl GetEnvTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GetEnvTool {
    fn name(&self) -> &str {
        "get_env"
    }

    fn description(&self) -> &str {
        "Read environment variables. Args: {\"name\": string (optional), \"names\": [string] (optional)}. With no args, lists all FORGE_* variables."
    }

    fn run(&self, args: &Value, _ctx: &ToolContext) -> Result<ToolResult> {
        if let Some(name) = optional_string_arg(args, "name") {
            return Ok(ToolResult::ok(
                std::env::var(&name).unwrap_or_else(|_| format!("(unset: {name})")),
            ));
        }
        if let Some(names) = args.get("names").and_then(Value::as_array) {
            let mut out = Vec::new();
            for n in names {
                if let Some(n) = n.as_str() {
                    let v = std::env::var(n).unwrap_or_else(|_| "(unset)".into());
                    out.push(format!("{n}={v}"));
                }
            }
            return Ok(ToolResult::ok(out.join("\n")));
        }
        // No args: list all FORGE_* variables.
        let mut vars: Vec<String> = std::env::vars()
            .filter(|(k, _)| k.starts_with("FORGE_"))
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        vars.sort();
        if vars.is_empty() {
            Ok(ToolResult::ok("(no FORGE_* variables set)"))
        } else {
            Ok(ToolResult::ok(vars.join("\n")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_a_single_variable() {
        std::env::set_var("FORGE_TEST_ENV", "hello");
        let tool = GetEnvTool::new();
        let ctx = ToolContext::new(std::env::temp_dir());
        let res = tool
            .run(&json!({ "name": "FORGE_TEST_ENV" }), &ctx)
            .unwrap();
        assert!(res.ok);
        assert_eq!(res.output, "hello");
        std::env::remove_var("FORGE_TEST_ENV");
    }

    #[test]
    fn unset_variable_reports_unset() {
        let tool = GetEnvTool::new();
        let ctx = ToolContext::new(std::env::temp_dir());
        let res = tool
            .run(&json!({ "name": "FORGE_DEFINITELY_UNSET_XYZ" }), &ctx)
            .unwrap();
        assert!(res.ok);
        assert!(res.output.contains("unset"));
    }

    #[test]
    fn lists_forge_vars() {
        std::env::set_var("FORGE_LIST_TEST", "1");
        let tool = GetEnvTool::new();
        let ctx = ToolContext::new(std::env::temp_dir());
        let res = tool.run(&json!({}), &ctx).unwrap();
        assert!(res.ok);
        assert!(res.output.contains("FORGE_LIST_TEST=1"));
        std::env::remove_var("FORGE_LIST_TEST");
    }
}
