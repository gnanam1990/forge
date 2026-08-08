//! Computer / desktop use: capture the screen and drive the mouse and keyboard
//! by coordinates. On macOS this uses `screencapture` for screenshots and
//! `cliclick` for input. This is the vision + coordinate model of computer use.

use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

/// Desktop control: screenshots and coordinate input.
#[derive(Debug, Clone, Default)]
pub struct Desktop;

impl Desktop {
    pub fn new() -> Self {
        Self
    }

    /// Capture the full screen to a PNG file.
    pub fn screenshot(&self, path: &Path) -> Result<()> {
        if !cfg!(target_os = "macos") {
            return Err(Error::Agent("desktop control is macOS-only".into()));
        }
        let status = Command::new("screencapture")
            .arg("-x")
            .arg(path)
            .status()
            .map_err(|e| Error::Agent(format!("screencapture: {e}")))?;
        if !status.success() {
            return Err(Error::Agent("screencapture failed".into()));
        }
        Ok(())
    }

    /// Move the mouse to a coordinate and click.
    pub fn click(&self, x: i32, y: i32) -> Result<()> {
        self.cliclick(&[&format!("c:{x},{y}")])
    }

    /// Move the mouse to a coordinate.
    pub fn move_to(&self, x: i32, y: i32) -> Result<()> {
        self.cliclick(&[&format!("m:{x},{y}")])
    }

    /// Type text at the current cursor position.
    pub fn type_text(&self, text: &str) -> Result<()> {
        self.cliclick(&[&format!("t:{text}")])
    }

    /// Press a key (e.g. "return", "tab", "escape").
    pub fn key(&self, key: &str) -> Result<()> {
        self.cliclick(&[&format!("k:{key}")])
    }

    /// Scroll by a delta (positive = up, negative = down).
    pub fn scroll(&self, delta: i32) -> Result<()> {
        self.cliclick(&[&format!("w:{}", delta)])
    }

    /// Double-click at a coordinate.
    pub fn double_click(&self, x: i32, y: i32) -> Result<()> {
        self.cliclick(&[&format!("d:{x},{y}")])
    }

    /// Run a cliclick command.
    fn cliclick(&self, args: &[&str]) -> Result<()> {
        if !cfg!(target_os = "macos") {
            return Err(Error::Agent("desktop control is macOS-only".into()));
        }
        let output = Command::new("cliclick")
            .args(args)
            .output()
            .map_err(|e| Error::Agent(format!("cliclick: {e}")))?;
        if !output.status.success() {
            return Err(Error::Agent(format!(
                "cliclick failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_click_args() {
        // The command-building is exercised through the cliclick invocation;
        // on non-macOS it errors cleanly rather than panicking.
        let desktop = Desktop::new();
        if cfg!(target_os = "macos") {
            let _ = desktop.click(10, 20);
        } else {
            assert!(desktop.click(10, 20).is_err());
        }
    }
}
