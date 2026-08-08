//! `ssh` — run a command on a remote host over SSH.

use std::process::Command;

use serde_json::Value;

use super::{optional_string_arg, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

#[derive(Default)]
pub struct SshTool;

impl SshTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for SshTool {
    fn name(&self) -> &str {
        "ssh"
    }

    fn description(&self) -> &str {
        "Run a command on a remote host over SSH. Args: {\"host\": string, \"command\": string, \"user\": string (optional), \"port\": number (optional)}."
    }

    fn permission(&self) -> Permission {
        Permission::Prompt
    }

    fn run(&self, args: &Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let host = string_arg(args, "host")?;
        let command = string_arg(args, "command")?;
        let user = optional_string_arg(args, "user").unwrap_or_default();
        let port = args.get("port").and_then(Value::as_u64).unwrap_or(22);

        let target = if user.is_empty() {
            host
        } else {
            format!("{user}@{host}")
        };
        let output = Command::new("ssh")
            .arg("-p")
            .arg(port.to_string())
            .arg(&target)
            .arg(&command)
            .output()?;
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
            Ok(ToolResult::err(format!(
                "ssh exited with {}\n{text}",
                output.status
            )))
        }
    }
}
