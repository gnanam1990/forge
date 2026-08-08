//! Sessions: persist and resume conversations.
//!
//! A session is a named, durable list of messages. The agent can resume a
//! session by continuing to append to its message list, so a long conversation
//! survives across runs.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::agent::Message;
use crate::error::{Error, Result};

/// A named, durable conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
    /// Unix timestamp of when the session was created.
    #[serde(default)]
    pub created_at: u64,
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            messages: Vec::new(),
            created_at: now_secs(),
        }
    }

    /// The number of messages in the session.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// A rough estimate of the total tokens across all messages.
    pub fn token_usage(&self) -> usize {
        self.messages.iter().map(|m| m.text_len()).sum()
    }

    /// Serialize the session to JSON.
    pub fn export(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Deserialize a session from JSON.
    pub fn import(json: &str) -> Result<Session> {
        serde_json::from_str(json).map_err(|e| Error::Config(format!("parse session: {e}")))
    }

    /// Save the session to `dir/<id>.json`.
    pub fn save(&self, dir: &Path) -> Result<()> {
        fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.json", self.id));
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(path, raw)?;
        Ok(())
    }

    /// Load a session by id from `dir`.
    pub fn load(dir: &Path, id: &str) -> Result<Session> {
        let path = dir.join(format!("{id}.json"));
        let raw = fs::read_to_string(&path)
            .map_err(|e| Error::Config(format!("read session {id}: {e}")))?;
        serde_json::from_str(&raw).map_err(|e| Error::Config(format!("parse session {id}: {e}")))
    }

    /// List the ids of all saved sessions in `dir`.
    pub fn list(dir: &Path) -> Result<Vec<String>> {
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                    ids.push(stem.to_string());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }
}

/// The default sessions directory: `~/.local/share/forge/sessions`.
pub fn default_sessions_dir() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("FORGE_SESSIONS_DIR") {
        if !p.trim().is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    let home = std::env::var("HOME").map_err(|_| Error::Config("HOME is not set".into()))?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("forge")
        .join("sessions"))
}

/// Current unix time in seconds.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Message;

    #[test]
    fn session_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = Session::new("abc");
        session.messages.push(Message::User("hello".into()));
        session.messages.push(Message::Assistant {
            content: "hi".into(),
            tool_calls: vec![],
        });
        session.save(dir.path()).unwrap();

        let loaded = Session::load(dir.path(), "abc").unwrap();
        assert_eq!(loaded.id, "abc");
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(Session::list(dir.path()).unwrap(), vec!["abc".to_string()]);
    }

    #[test]
    fn list_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Session::list(dir.path()).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn token_usage_estimates_text() {
        let mut session = Session::new("tok");
        // "aaaaaaaaaaaa" = 12 chars -> 3 tokens each; two messages -> 6 total.
        session.messages.push(Message::User("aaaaaaaaaaaa".into()));
        session.messages.push(Message::User("aaaaaaaaaaaa".into()));
        assert_eq!(session.token_usage(), 6);
    }
}
