//! `web_fetch` — fetch a URL and return its text content.

use serde_json::Value;

use super::{string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

/// Cap on how much fetched text is returned.
const MAX_BYTES: usize = 64 * 1024;

#[derive(Default)]
pub struct WebFetchTool;

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch a URL and return its text content. Args: {\"url\": string}."
    }

    fn permission(&self) -> Permission {
        Permission::Prompt
    }

    fn run(&self, args: &Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let url = string_arg(args, "url")?;
        let client = reqwest::blocking::Client::new();
        let response = match client.get(&url).send() {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::err(format!("request failed: {e}"))),
        };
        if !response.status().is_success() {
            let status = response.status();
            return Ok(ToolResult::err(format!("HTTP {status}")));
        }
        let bytes = match response.bytes() {
            Ok(b) => b,
            Err(e) => return Ok(ToolResult::err(format!("read failed: {e}"))),
        };
        let truncated = bytes.len() > MAX_BYTES;
        let text = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_BYTES)]).into_owned();
        let mut output = text;
        if truncated {
            output.push_str(&format!(
                "\n[truncated: response exceeds {} bytes]",
                MAX_BYTES
            ));
        }
        Ok(ToolResult::ok(output))
    }
}
