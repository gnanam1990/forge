//! Telemetry: a minimal usage tracker that appends JSON events to a file.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::error::{Error, Result};

/// Records usage events to a JSONL file.
#[derive(Debug, Clone)]
pub struct Telemetry {
    enabled: bool,
    path: PathBuf,
}

impl Telemetry {
    pub fn new(enabled: bool, path: PathBuf) -> Self {
        Self { enabled, path }
    }

    /// Record an event. A no-op when disabled.
    pub fn record(&self, event: &str, data: Value) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let line = json!({
            "event": event,
            "ts": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            "data": data,
        });
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| Error::Config(format!("open telemetry: {e}")))?;
        writeln!(file, "{line}")?;
        Ok(())
    }
}

/// The default telemetry file: `~/.local/share/forge/telemetry.jsonl`.
pub fn default_telemetry_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| Error::Config("HOME is not set".into()))?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("forge")
        .join("telemetry.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_is_noop() {
        let telemetry = Telemetry::new(false, std::env::temp_dir().join("t.jsonl"));
        assert!(telemetry.record("test", json!({})).is_ok());
    }

    #[test]
    fn records_an_event() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.jsonl");
        let telemetry = Telemetry::new(true, path.clone());
        telemetry.record("run", json!({ "turns": 3 })).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"event\":\"run\""));
        assert!(raw.contains("\"turns\":3"));
    }
}
