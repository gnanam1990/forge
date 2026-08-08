//! A sandbox that runs shell commands in isolation: a fresh working directory,
//! a minimal environment, and (on macOS) an optional `sandbox-exec` profile
//! that denies network access.

use std::path::PathBuf;
use std::process::Command;

use crate::error::Result;

/// The result of a sandboxed command.
#[derive(Debug, Clone)]
pub struct SandboxResult {
    pub output: String,
    pub exit_code: i32,
}

/// Sandbox restrictions.
#[derive(Debug, Clone, Default)]
pub struct SandboxConfig {
    /// Deny all network access.
    pub deny_network: bool,
    /// Deny writes outside the sandbox working directory.
    pub deny_write: bool,
}

/// Runs commands in isolation.
#[derive(Debug, Clone)]
pub struct Sandbox {
    enabled: bool,
    config: SandboxConfig,
    work_dir: PathBuf,
}

impl Sandbox {
    pub fn new(enabled: bool) -> Self {
        Self::with_config(enabled, SandboxConfig::default())
    }

    pub fn with_config(enabled: bool, config: SandboxConfig) -> Self {
        Self {
            enabled,
            config,
            work_dir: std::env::temp_dir().join(format!("forge-sandbox-{}", std::process::id())),
        }
    }

    /// Run a command in the sandbox. Returns combined output and exit code.
    pub fn run(&self, command: &str) -> Result<SandboxResult> {
        std::fs::create_dir_all(&self.work_dir)?;

        // A minimal environment: only PATH and HOME, nothing else leaks in.
        let env: Vec<(String, String)> = vec![
            (
                "PATH".to_string(),
                std::env::var("PATH").unwrap_or_default(),
            ),
            (
                "HOME".to_string(),
                std::env::var("HOME").unwrap_or_default(),
            ),
        ];

        // Write the command to a script so it can be wrapped cleanly.
        let script = self.work_dir.join("cmd.sh");
        std::fs::write(&script, command)?;

        // On macOS, wrap with sandbox-exec to enforce the configured
        // restrictions when enabled.
        let full = if self.enabled && cfg!(target_os = "macos") {
            let mut profile = String::from("(version 1)\n(allow default)\n");
            if self.config.deny_network {
                profile.push_str("(deny network*)\n");
            }
            if self.config.deny_write {
                profile.push_str("(deny file-write*)\n");
            }
            let profile_path = self.work_dir.join("sandbox.sb");
            std::fs::write(&profile_path, profile)?;
            format!(
                "sandbox-exec -f {} sh {}",
                profile_path.display(),
                script.display()
            )
        } else {
            format!("sh {}", script.display())
        };

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(&full)
            .current_dir(&self.work_dir)
            .env_clear()
            .envs(env);

        let output = cmd.output()?;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        if !output.stderr.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&String::from_utf8_lossy(&output.stderr));
        }
        Ok(SandboxResult {
            output: text,
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_in_isolated_dir() {
        let sandbox = Sandbox::new(false);
        let result = sandbox.run("pwd").unwrap();
        assert!(result.output.contains("forge-sandbox"));
    }

    #[test]
    fn env_is_isolated() {
        // A secret in the parent environment must not leak into the sandbox.
        std::env::set_var("FORGE_SECRET", "do-not-leak");
        let sandbox = Sandbox::new(false);
        let result = sandbox.run("env").unwrap();
        assert!(!result.output.contains("FORGE_SECRET"));
    }
}
