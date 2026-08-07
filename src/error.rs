//! Error types shared across forge.

use std::path::PathBuf;

/// The unified error type for forge. Every failure in the library surfaces as
/// one of these so the CLI can render a consistent, actionable message.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("tool error: {0}")]
    Tool(String),

    #[error("agent error: {0}")]
    Agent(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("path {0} is outside the workspace {1}")]
    OutsideWorkspace(PathBuf, PathBuf),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
