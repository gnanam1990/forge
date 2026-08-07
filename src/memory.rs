//! Memory: a durable key-value store of facts the agent learns across sessions.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// A durable store of key-value facts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Memory {
    facts: HashMap<String, String>,
    #[serde(skip)]
    path: PathBuf,
}

impl Memory {
    /// Create an in-memory store (not persisted until `save`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a store from a JSON file, or an empty store if it does not exist.
    pub fn load(path: PathBuf) -> Result<Self> {
        let mut memory = if path.exists() {
            let raw = fs::read_to_string(&path)
                .map_err(|e| Error::Config(format!("read memory {}: {e}", path.display())))?;
            serde_json::from_str(&raw)
                .map_err(|e| Error::Config(format!("parse memory {}: {e}", path.display())))?
        } else {
            Self::default()
        };
        memory.path = path;
        Ok(memory)
    }

    /// Remember a fact.
    pub fn remember(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.facts.insert(key.into(), value.into());
    }

    /// Recall a fact.
    pub fn recall(&self, key: &str) -> Option<&str> {
        self.facts.get(key).map(String::as_str)
    }

    /// All facts, sorted by key.
    pub fn all(&self) -> Vec<(String, String)> {
        let mut facts: Vec<(String, String)> = self.facts.clone().into_iter().collect();
        facts.sort();
        facts
    }

    /// Persist to the configured path.
    pub fn save(&self) -> Result<()> {
        if self.path.as_os_str().is_empty() {
            return Err(Error::Config("memory has no path; use Memory::load".into()));
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(&self.path, raw)?;
        Ok(())
    }
}

/// The default memory file: `~/.local/share/forge/memory.json`.
pub fn default_memory_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| Error::Config("HOME is not set".into()))?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("forge")
        .join("memory.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remember_and_recall() {
        let mut memory = Memory::new();
        memory.remember("lang", "rust");
        assert_eq!(memory.recall("lang"), Some("rust"));
        assert_eq!(memory.recall("missing"), None);
    }

    #[test]
    fn persists_and_loads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("memory.json");
        {
            let mut memory = Memory::load(path.clone()).unwrap();
            memory.remember("key", "value");
            memory.save().unwrap();
        }
        let loaded = Memory::load(path).unwrap();
        assert_eq!(loaded.recall("key"), Some("value"));
    }
}
