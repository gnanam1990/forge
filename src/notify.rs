//! Notifications: send a completion alert. On macOS this uses a native
//! notification via `osascript`; elsewhere it falls back to stderr.

use std::process::Command;

use crate::error::{Error, Result};

/// Sends completion notifications.
#[derive(Debug, Clone)]
pub struct Notifier {
    enabled: bool,
}

impl Notifier {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Send a notification. Returns Ok even when disabled (a no-op).
    pub fn notify(&self, title: &str, message: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        if cfg!(target_os = "macos") {
            let script = format!(
                "display notification {} with title {}",
                shell_quote(message),
                shell_quote(title)
            );
            let status = Command::new("osascript").arg("-e").arg(&script).status()?;
            if !status.success() {
                return Err(Error::Agent("osascript notification failed".into()));
            }
        } else {
            eprintln!("[forge] {title}: {message}");
        }
        Ok(())
    }
}

/// Quote a string for AppleScript.
fn shell_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_is_noop() {
        let notifier = Notifier::new(false);
        assert!(notifier.notify("t", "m").is_ok());
    }
}
