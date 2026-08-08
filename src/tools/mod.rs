//! The tool system: a `Tool` trait, a shared `ToolContext`, and a `Registry`
//! that tools are looked up from by name. Tools are the only way the agent
//! touches the outside world, so every tool enforces the workspace boundary.

pub mod apply_patch;
pub mod ask_user;
pub mod bash;
pub mod edit_file;
pub mod git;
pub mod glob;
pub mod grep;
pub mod list_directory;
pub mod read_file;
pub mod search;
pub mod ssh;
pub mod terminal;
pub mod web_fetch;
pub mod write_file;

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{Error, Result};
use crate::permission::Permission;

/// Per-call context handed to every tool. Carries the workspace root the tool
/// must stay inside.
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub workspace_root: PathBuf,
}

impl ToolContext {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

/// The result of running a tool. `ok` distinguishes a successful run from a
/// tool-level failure; `output` is the human-readable text returned to the
/// agent, and `error` carries the failure message when `ok` is false.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub ok: bool,
    pub output: String,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn ok(output: impl Into<String>) -> Self {
        Self {
            ok: true,
            output: output.into(),
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            ok: false,
            output: message.clone(),
            error: Some(message),
        }
    }
}

/// A single tool the agent can call. Implementations must be `Send + Sync` so
/// the registry can be shared across threads.
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult>;

    /// The safety level this tool declares. Read-only tools default to `Allow`;
    /// mutating or network tools override to `Prompt`.
    fn permission(&self) -> Permission {
        Permission::Allow
    }
}

/// A registry of tools, looked up by name.
#[derive(Default)]
pub struct Registry {
    tools: Vec<Box<dyn Tool>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool. Duplicate names are replaced.
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.retain(|t| t.name() != tool.name());
        self.tools.push(tool);
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    /// All registered tool names, sorted.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tools.iter().map(|t| t.name().to_string()).collect();
        names.sort();
        names
    }

    /// Build the default registry with the built-in tools.
    pub fn builtin() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(read_file::ReadFileTool::new()));
        registry.register(Box::new(write_file::WriteFileTool::new()));
        registry.register(Box::new(edit_file::EditFileTool::new()));
        registry.register(Box::new(list_directory::ListDirectoryTool::new()));
        registry.register(Box::new(bash::BashTool::new()));
        registry.register(Box::new(glob::GlobTool::new()));
        registry.register(Box::new(grep::GrepTool::new()));
        registry.register(Box::new(web_fetch::WebFetchTool::new()));
        registry.register(Box::new(ask_user::AskUserTool::new()));
        registry.register(Box::new(git::GitStatusTool::new()));
        registry.register(Box::new(git::GitDiffTool::new()));
        registry.register(Box::new(git::GitCommitTool::new()));
        registry.register(Box::new(apply_patch::ApplyPatchTool::new()));
        registry.register(Box::new(search::SearchTool::new()));
        registry.register(Box::new(terminal::TerminalTool::new()));
        registry.register(Box::new(ssh::SshTool::new()));
        registry
    }

    /// Build the default registry plus the memory tools backed by a store.
    pub fn with_memory(memory: std::sync::Arc<std::sync::Mutex<crate::memory::Memory>>) -> Self {
        let mut registry = Self::builtin();
        registry.register(Box::new(crate::memory::RememberTool::new(memory.clone())));
        registry.register(Box::new(crate::memory::RecallTool::new(memory)));
        registry
    }
}

/// Resolve a user-supplied path against the workspace root, refusing any path
/// that escapes the workspace. This is the single enforcement point every
/// file-touching tool goes through.
pub fn resolve_in_workspace(workspace_root: &Path, raw: &str) -> Result<PathBuf> {
    let root = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    let candidate = PathBuf::from(raw);
    let candidate = if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    };
    // Prefer the canonical path (resolves symlinks); fall back to a lexical
    // normalization so a `..` escape is still caught when the target does not
    // exist (canonicalize fails on missing files).
    let canonical = candidate
        .canonicalize()
        .unwrap_or_else(|_| lexical_normalize(&candidate));
    if !canonical.starts_with(&root) {
        return Err(Error::OutsideWorkspace(canonical, root));
    }
    Ok(canonical)
}

/// Resolve `.` and `..` components lexically, without touching the filesystem.
/// Used to catch path escapes for paths that do not exist yet.
fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Read a string argument from a JSON object, or return an invalid-args error.
pub fn string_arg(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| Error::InvalidArgs(format!("missing string argument \"{key}\"")))
}

/// Read an optional string argument.
pub fn optional_string_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_rejects_escape() {
        let root = tempfile::tempdir().unwrap();
        let res = resolve_in_workspace(root.path(), "../etc/passwd");
        assert!(matches!(res, Err(Error::OutsideWorkspace(_, _))));
    }

    #[test]
    fn resolve_accepts_inside() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("a.txt");
        std::fs::write(&file, "x").unwrap();
        let res = resolve_in_workspace(root.path(), "a.txt").unwrap();
        assert_eq!(res, file.canonicalize().unwrap());
    }
}
