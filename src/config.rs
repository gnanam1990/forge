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
    /// Maximum number of agent turns before the loop stops.
    pub max_turns: Option<usize>,
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
}
