//! `terminal` — run a command in the workspace and return its output, plus a
//! persistent terminal session that keeps a shell alive across commands.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::Value;

use super::{string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

/// A persistent shell session: a long-lived `sh` process with piped stdin and
/// stdout, so state (cwd, env, variables) survives across commands.
pub struct TerminalSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl TerminalSession {
    /// Spawn a persistent shell.
    pub fn spawn() -> Result<Self> {
        let mut child = Command::new("sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| crate::error::Error::Agent("no stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| crate::error::Error::Agent("no stdout".into()))?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    /// Send a command to the session.
    pub fn send(&mut self, command: &str) -> Result<()> {
        self.stdin.write_all(command.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Read all currently-available output from the session.
    pub fn read(&mut self) -> Result<String> {
        let mut out = String::new();
        loop {
            let mut buf = String::new();
            let n = self.stdout.read_line(&mut buf)?;
            if n == 0 {
                break;
            }
            out.push_str(&buf);
            if !buf.ends_with('\n') {
                break;
            }
        }
        Ok(out)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Default)]
pub struct TerminalTool;

impl TerminalTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for TerminalTool {
    fn name(&self) -> &str {
        "terminal"
    }

    fn description(&self) -> &str {
        "Run a command in the workspace and return its output. Args: {\"command\": string}."
    }

    fn permission(&self) -> Permission {
        Permission::Prompt
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let command = string_arg(args, "command")?;
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .current_dir(&ctx.workspace_root)
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
                "command exited with {}\n{text}",
                output.status
            )))
        }
    }
}
