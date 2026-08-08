//! `search` — a simple code index: tokenize files and find which files contain
//! a given identifier or word.

use std::collections::HashMap;

use serde_json::Value;
use walkdir::WalkDir;

use super::{string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;

/// A basic in-memory code index: token -> set of files containing it.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CodeIndex {
    index: HashMap<String, Vec<String>>,
}

impl CodeIndex {
    pub fn build(root: &std::path::Path) -> Self {
        let mut index: HashMap<String, Vec<String>> = HashMap::new();
        for entry in WalkDir::new(root)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            let rel = entry
                .path()
                .strip_prefix(root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .into_owned();
            for token in tokenize(&content) {
                index.entry(token).or_default().push(rel.clone());
            }
        }
        Self { index }
    }

    /// Files containing the token, deduplicated.
    pub fn search(&self, token: &str) -> Vec<String> {
        let mut files = self.index.get(token).cloned().unwrap_or_default();
        files.sort();
        files.dedup();
        files
    }

    /// Files whose tokens fuzzy-match the query (subsequence match), used as a
    /// fallback when an exact search finds nothing.
    pub fn fuzzy_search(&self, query: &str) -> Vec<String> {
        let mut files = Vec::new();
        for (token, paths) in &self.index {
            if fuzzy_match(query, token) {
                files.extend(paths.iter().cloned());
            }
        }
        files.sort();
        files.dedup();
        files
    }

    /// Persist the index to a file.
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string(self)?;
        std::fs::write(path, raw)?;
        Ok(())
    }

    /// Load an index from a file, or None if it does not exist.
    pub fn load(path: &std::path::Path) -> Option<Self> {
        let raw = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&raw).ok()
    }
}

/// Split text into lowercase alphanumeric tokens.
fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch);
        } else if !current.is_empty() {
            tokens.push(current.to_lowercase());
            current.clear();
        }
    }
    if !current.is_empty() {
        tokens.push(current.to_lowercase());
    }
    tokens
}

/// A simple subsequence fuzzy match: every character of `query` appears in
/// `text` in order.
fn fuzzy_match(query: &str, text: &str) -> bool {
    let query: Vec<char> = query.chars().collect();
    if query.is_empty() {
        return true;
    }
    let mut qi = 0usize;
    for c in text.chars() {
        if c == query[qi] {
            qi += 1;
            if qi == query.len() {
                return true;
            }
        }
    }
    false
}

#[derive(Default)]
pub struct SearchTool;

impl SearchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "Search the code index for files containing an identifier. Args: {\"query\": string}."
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let query = string_arg(args, "query")?;
        // Use a persistent index cached under the workspace, rebuilt only when
        // the cache is missing.
        let cache = ctx.workspace_root.join(".forge").join("index.json");
        let index = match CodeIndex::load(&cache) {
            Some(index) => index,
            None => {
                let index = CodeIndex::build(&ctx.workspace_root);
                let _ = index.save(&cache);
                index
            }
        };
        let files = index.search(&query.to_lowercase());
        let files = if files.is_empty() {
            index.fuzzy_search(&query.to_lowercase())
        } else {
            files
        };
        if files.is_empty() {
            return Ok(ToolResult::ok("no matches"));
        }
        Ok(ToolResult::ok(files.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes() {
        let tokens = tokenize("fn main() { let foo_bar = 1; }");
        assert!(tokens.contains(&"main".to_string()));
        assert!(tokens.contains(&"foo_bar".to_string()));
    }

    #[test]
    fn index_finds_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "fn helper() {}").unwrap();
        std::fs::write(dir.path().join("b.rs"), "fn other() {}").unwrap();
        let index = CodeIndex::build(dir.path());
        assert_eq!(index.search("helper"), vec!["a.rs".to_string()]);
        assert!(index.search("missing").is_empty());
    }
}
