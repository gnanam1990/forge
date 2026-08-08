//! Configuration loading for forge.
//!
//! forge reads a small JSON config file (default `~/.config/forge/config.json`,
//! overridable with `FORGE_CONFIG`). The config carries the workspace root and
//! the model provider settings. Everything is optional so the tool works out of
//! the box for tests and local use.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The on-disk configuration schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Absolute path to the workspace the agent is allowed to touch.
    pub workspace: Option<PathBuf>,
    /// Provider settings for the model backend.
    pub provider: ProviderConfig,
    /// Additional saved providers (for switching models).
    #[serde(default)]
    pub saved_providers: Vec<ProviderConfig>,
    /// Maximum number of agent turns before the loop stops.
    pub max_turns: Option<usize>,
    /// MCP servers to auto-register at startup.
    pub mcp_servers: Vec<McpServerConfig>,
    /// Directory of JSON plugin files to load at startup.
    pub plugins_dir: Option<PathBuf>,
    /// Hooks to install at startup.
    pub hooks: Vec<HookConfig>,
    /// Whether to record telemetry events.
    pub telemetry: bool,
    /// Command aliases: name -> command.
    #[serde(default)]
    pub aliases: std::collections::HashMap<String, String>,
    /// Project command overrides for build/test/lint.
    #[serde(default)]
    pub commands: CommandsConfig,
}

/// Project command overrides used by `forge build`, `forge test`, and
/// `forge lint`. Each is an optional shell command; when unset, forge falls
/// back to a sensible default for the detected project type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CommandsConfig {
    /// Command to build the project (e.g. `cargo build`).
    pub build: Option<String>,
    /// Command to run the project's tests (e.g. `cargo test`).
    pub test: Option<String>,
    /// Command to lint the project (e.g. `cargo clippy`).
    pub lint: Option<String>,
}

impl CommandsConfig {
    /// Resolve the effective command for a step, falling back to a default
    /// detected from the workspace contents.
    pub fn resolve(&self, step: &str, workspace: &Path) -> String {
        if let Some(cmd) = match step {
            "build" => self.build.as_deref(),
            "test" => self.test.as_deref(),
            "lint" => self.lint.as_deref(),
            _ => None,
        } {
            return cmd.to_string();
        }
        default_command(step, workspace)
    }
}

/// Pick a default command for a step based on the project type detected in the
/// workspace (Cargo, npm, pnpm, yarn, Python, or a plain shell fallback).
fn default_command(step: &str, workspace: &Path) -> String {
    let has = |name: &str| workspace.join(name).exists();
    let cargo = has("Cargo.toml");
    let npm = has("package.json");
    let pnpm = has("pnpm-lock.yaml");
    let yarn = has("yarn.lock");
    let python = has("pyproject.toml") || has("requirements.txt") || has("setup.py");
    match step {
        "build" => {
            if cargo {
                "cargo build".to_string()
            } else if pnpm {
                "pnpm build".to_string()
            } else if yarn {
                "yarn build".to_string()
            } else if npm {
                "npm run build".to_string()
            } else if python {
                "python -m build".to_string()
            } else {
                "make build".to_string()
            }
        }
        "test" => {
            if cargo {
                "cargo test".to_string()
            } else if pnpm {
                "pnpm test".to_string()
            } else if yarn {
                "yarn test".to_string()
            } else if npm {
                "npm test".to_string()
            } else if python {
                "python -m pytest".to_string()
            } else {
                "make test".to_string()
            }
        }
        "lint" => {
            if cargo {
                "cargo clippy --all-targets".to_string()
            } else if pnpm {
                "pnpm lint".to_string()
            } else if yarn {
                "yarn lint".to_string()
            } else if npm {
                "npm run lint".to_string()
            } else if python {
                "python -m ruff check .".to_string()
            } else {
                "make lint".to_string()
            }
        }
        _ => String::new(),
    }
}

/// An MCP server to connect to at startup.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// A hook to install at startup.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookConfig {
    pub name: String,
    /// The shell command to run before a tool call.
    pub before: Option<String>,
    /// The shell command to run after a tool call.
    pub after: Option<String>,
}

/// Model provider settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    /// Base URL of an OpenAI-compatible chat completions endpoint.
    pub base_url: Option<String>,
    /// Model identifier, e.g. "gpt-4o-mini".
    pub model: Option<String>,
    /// API key. Prefer the `FORGE_API_KEY` environment variable.
    pub api_key: Option<String>,
}

impl Config {
    /// Load configuration from the default location, or an empty config if none
    /// exists. A present-but-malformed file is a hard error.
    pub fn load() -> Result<Config> {
        let path = config_path()?;
        Self::load_from(&path)
    }

    /// Load configuration from an explicit path.
    pub fn load_from(path: &Path) -> Result<Config> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let raw = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("read {}: {e}", path.display())))?;
        serde_json::from_str(&raw)
            .map_err(|e| Error::Config(format!("parse {}: {e}", path.display())))
    }

    /// The effective workspace root: the configured one, or the current
    /// directory when unset.
    pub fn workspace_root(&self) -> PathBuf {
        self.workspace
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// Validate the configuration. Returns an error naming the first problem.
    pub fn validate(&self) -> Result<()> {
        if let Some(ws) = &self.workspace {
            if !ws.is_dir() {
                return Err(Error::Config(format!(
                    "workspace {} is not a directory",
                    ws.display()
                )));
            }
        }
        if let Some(base) = &self.provider.base_url {
            if !base.starts_with("http://") && !base.starts_with("https://") {
                return Err(Error::Config(format!(
                    "provider.base_url must start with http:// or https://, got {base}"
                )));
            }
        }
        if let Some(model) = &self.provider.model {
            if model.trim().is_empty() {
                return Err(Error::Config("provider.model must not be empty".into()));
            }
        }
        if let Some(turns) = self.max_turns {
            if turns == 0 {
                return Err(Error::Config("max_turns must be >= 1".into()));
            }
        }
        Ok(())
    }
}

/// Resolve the config file path: `$FORGE_CONFIG` if set, otherwise
/// `~/.config/forge/config.json`.
pub(crate) fn config_path() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("FORGE_CONFIG") {
        if !p.trim().is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    let home = std::env::var("HOME")
        .map_err(|_| Error::Config("HOME is not set; set FORGE_CONFIG instead".into()))?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("forge")
        .join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn missing_config_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::load_from(&dir.path().join("nope.json")).unwrap();
        assert!(cfg.workspace.is_none());
        assert!(cfg.provider.model.is_none());
    }

    #[test]
    fn malformed_config_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, "{ not json").unwrap();
        assert!(Config::load_from(&path).is_err());
    }

    #[test]
    fn parses_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(
            &path,
            r#"{"workspace":"/tmp/w","provider":{"model":"gpt-4o-mini"},"max_turns":5}"#,
        )
        .unwrap();
        let cfg = Config::load_from(&path).unwrap();
        assert_eq!(cfg.workspace.as_deref(), Some(Path::new("/tmp/w")));
        assert_eq!(cfg.provider.model.as_deref(), Some("gpt-4o-mini"));
        assert_eq!(cfg.max_turns, Some(5));
    }

    #[test]
    fn validate_rejects_bad_base_url() {
        let cfg = Config {
            provider: ProviderConfig {
                base_url: Some("ftp://bad".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_zero_turns() {
        let cfg = Config {
            max_turns: Some(0),
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_accepts_default() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn default_command_detects_cargo() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\n").unwrap();
        let cmds = CommandsConfig::default();
        assert_eq!(cmds.resolve("build", dir.path()), "cargo build");
        assert_eq!(cmds.resolve("test", dir.path()), "cargo test");
        assert_eq!(
            cmds.resolve("lint", dir.path()),
            "cargo clippy --all-targets"
        );
    }

    #[test]
    fn default_command_detects_npm() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        let cmds = CommandsConfig::default();
        assert_eq!(cmds.resolve("build", dir.path()), "npm run build");
        assert_eq!(cmds.resolve("test", dir.path()), "npm test");
        assert_eq!(cmds.resolve("lint", dir.path()), "npm run lint");
    }

    #[test]
    fn explicit_command_overrides_default() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\n").unwrap();
        let cmds = CommandsConfig {
            build: Some("make build".into()),
            ..Default::default()
        };
        assert_eq!(cmds.resolve("build", dir.path()), "make build");
        assert_eq!(cmds.resolve("test", dir.path()), "cargo test");
    }
}
