//! `bash` — run a shell command with a bounded timeout.

use std::process::Command;

use serde_json::Value;

use super::{string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;

/// Default timeout for a shell command, in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Default)]
pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Run a shell command. Args: {\"command\": string, \"timeout_secs\": number (optional)}."
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let command = string_arg(args, "command")?;
        let timeout = args
            .get("timeout_secs")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        // Run the command in the workspace directory.
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .current_dir(&ctx.workspace_root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Wait with a timeout by polling. A simpler approach would block, but a
        // hung command must not hang the agent.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout);
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(ToolResult::err(format!(
                    "command timed out after {timeout}s: {command}"
                )));
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        };

        let output = child.wait_with_output()?;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        if !output.stderr.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&String::from_utf8_lossy(&output.stderr));
        }
        if status.success() {
            Ok(ToolResult::ok(text))
        } else {
            Ok(ToolResult::err(format!(
                "command exited with {status}\n{text}"
            )))
        }
    }
}
