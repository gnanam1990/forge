//! `web_search` — search the web via the DuckDuckGo Instant Answer API (no API
//! key required) and return the top results.

use serde_json::Value;

use super::{string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

/// Cap on how many results are returned.
const MAX_RESULTS: usize = 10;

#[derive(Default)]
pub struct WebSearchTool;

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web and return the top results. Args: {\"query\": string}."
    }

    fn permission(&self) -> Permission {
        Permission::Prompt
    }

    fn run(&self, args: &Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let query = string_arg(args, "query")?;
        let client = reqwest::blocking::Client::new();
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1",
            urlencode(&query)
        );
        let response = match client.get(&url).send() {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::err(format!("search failed: {e}"))),
        };
        let value: Value = match response.json() {
            Ok(v) => v,
            Err(e) => return Ok(ToolResult::err(format!("parse search: {e}"))),
        };

        let mut lines = Vec::new();
        if let Some(answer) = value.get("AbstractText").and_then(Value::as_str) {
            if !answer.is_empty() {
                lines.push(format!("abstract: {answer}"));
            }
        }
        if let Some(definition) = value.get("Definition").and_then(Value::as_str) {
            if !definition.is_empty() {
                lines.push(format!("definition: {definition}"));
            }
        }
        if let Some(related) = value.get("RelatedTopics").and_then(Value::as_array) {
            for topic in related.iter().take(MAX_RESULTS) {
                if let Some(text) = topic.get("Text").and_then(Value::as_str) {
                    let url = topic.get("FirstURL").and_then(Value::as_str).unwrap_or("");
                    lines.push(format!("- {text}\n  {url}"));
                }
            }
        }
        if lines.is_empty() {
            return Ok(ToolResult::ok("no results"));
        }
        Ok(ToolResult::ok(lines.join("\n")))
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencodes_query() {
        assert_eq!(urlencode("a b"), "a%20b");
    }
}
