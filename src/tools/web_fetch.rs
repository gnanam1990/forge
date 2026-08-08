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
        let client = reqwest::blocking::Client::builder()
            .user_agent("forge/0.1 (coding agent)")
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| crate::error::Error::Tool(format!("build client: {e}")))?;
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
        let raw = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_BYTES)]).into_owned();
        let text = html_to_text(&raw);
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

/// A light HTML-to-text conversion: strips tags and decodes common entities.
fn html_to_text(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    let mut skip_depth = 0usize;
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '<' => {
                let rest: String = chars.clone().take(7).collect();
                if rest.starts_with("script") || rest.starts_with("style") {
                    skip_depth += 1;
                } else if rest.starts_with("/script") || rest.starts_with("/style") {
                    skip_depth = skip_depth.saturating_sub(1);
                }
                in_tag = true;
            }
            '>' => in_tag = false,
            _ if in_tag || skip_depth > 0 => {}
            '&' => {
                let entity: String = chars.clone().take(6).collect();
                let decoded = match entity.as_str() {
                    "amp;" => "&",
                    "lt;" => "<",
                    "gt;" => ">",
                    "quot;" => "\"",
                    "nbsp;" => " ",
                    _ => "&",
                };
                out.push_str(decoded);
                for _ in 0..decoded.len().saturating_sub(1) {
                    chars.next();
                }
            }
            c => out.push(c),
        }
    }
    // Collapse runs of whitespace.
    let mut collapsed = String::new();
    let mut last_space = false;
    for c in out.chars() {
        if c.is_whitespace() {
            if !last_space {
                collapsed.push(' ');
                last_space = true;
            }
        } else {
            collapsed.push(c);
            last_space = false;
        }
    }
    collapsed.trim().to_string()
}
