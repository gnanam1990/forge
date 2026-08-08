//! `terminal` — run a command in the workspace and return its output, plus a
//! persistent terminal session backed by a real pseudo-terminal (pty).

use std::io::{Read, Write};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde_json::Value;

use super::{string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

/// A persistent terminal session backed by a real pty, so interactive programs
/// (editors, TUI apps) work and state survives across commands.
pub struct TerminalSession {
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
}

impl TerminalSession {
    /// Spawn a persistent shell in a pty.
    pub fn spawn() -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| crate::error::Error::Agent(format!("openpty: {e}")))?;
        let cmd = CommandBuilder::new("sh");
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| crate::error::Error::Agent(format!("spawn shell: {e}")))?;
        drop(pair.slave);
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| crate::error::Error::Agent(format!("pty reader: {e}")))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| crate::error::Error::Agent(format!("pty writer: {e}")))?;
        Ok(Self {
            _child: child,
            reader,
            writer,
        })
    }

    /// Send a command to the session.
    pub fn send(&mut self, command: &str) -> Result<()> {
        self.writer.write_all(command.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }

    /// Read all currently-available output from the session.
    pub fn read(&mut self) -> Result<String> {
        let mut buf = [0u8; 4096];
        let mut out = String::new();
        loop {
            match self.reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => out.push_str(&String::from_utf8_lossy(&buf[..n])),
            }
        }
        Ok(out)
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
        let output = std::process::Command::new("sh")
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
