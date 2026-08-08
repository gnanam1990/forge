//! `apply_patch` — apply a unified diff to the workspace with a built-in
//! parser (no external `patch` dependency).

use serde_json::Value;

use super::{resolve_in_workspace, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;
use crate::permission::Permission;

#[derive(Default)]
pub struct ApplyPatchTool;

impl ApplyPatchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Apply a unified diff to the workspace. Args: {\"patch\": string}."
    }

    fn permission(&self) -> Permission {
        Permission::Prompt
    }

    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let patch = string_arg(args, "patch")?;
        match apply_unified_diff(&ctx.workspace_root, &patch) {
            Ok(summary) => Ok(ToolResult::ok(summary)),
            Err(e) => Ok(ToolResult::err(format!("patch failed: {e}"))),
        }
    }
}

/// Apply a unified diff rooted at `root`. Returns a summary of what changed.
pub fn apply_unified_diff(root: &std::path::Path, patch: &str) -> Result<String> {
    let mut files: Vec<(String, Vec<String>)> = Vec::new();
    let mut current: Option<(String, Vec<String>)> = None;

    for line in patch.lines() {
        if let Some(header) = line.strip_prefix("+++ b/") {
            if let Some((_, body)) = current.take() {
                files.push((header.to_string(), body));
            }
            current = Some((header.to_string(), Vec::new()));
        } else if let Some((_, body)) = current.as_mut() {
            body.push(line.to_string());
        }
    }
    if let Some((name, body)) = current.take() {
        files.push((name, body));
    }

    if files.is_empty() {
        return Err(crate::error::Error::InvalidArgs(
            "no file headers in patch".into(),
        ));
    }

    let mut summary = String::new();
    for (name, body) in files {
        let path = resolve_in_workspace(root, &name)?;
        let original = if path.exists() {
            std::fs::read_to_string(&path)?
        } else {
            String::new()
        };
        let updated = apply_hunks(&original, &body)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, updated)?;
        summary.push_str(&format!("patched {name}\n"));
    }
    Ok(summary)
}

/// Apply the hunks in a diff body to the original text.
fn apply_hunks(original: &str, body: &[String]) -> Result<String> {
    let mut lines: Vec<&str> = original.lines().collect();
    let mut i = 0usize;
    let mut idx = 0usize;
    while i < body.len() {
        let line = &body[i];
        if let Some(header) = line.strip_prefix("@@ ") {
            // Parse the new-file start line: @@ -a,b +c,d @@
            let start = parse_hunk_start(header)?;
            i += 1;
            // Apply the hunk body until the next @@ or end.
            let mut applied = 0usize;
            while i < body.len() && !body[i].starts_with("@@ ") {
                let l = &body[i];
                if let Some(text) = l.strip_prefix('-') {
                    // Remove a line at the current position.
                    if idx < lines.len() && lines[idx] == text {
                        lines.remove(idx);
                    } else {
                        return Err(crate::error::Error::InvalidArgs(format!(
                            "context mismatch at line {}",
                            idx + 1
                        )));
                    }
                } else if let Some(text) = l.strip_prefix('+') {
                    lines.insert(idx, text);
                    idx += 1;
                } else if let Some(text) = l.strip_prefix(' ') {
                    if idx < lines.len() && lines[idx] == text {
                        idx += 1;
                    } else {
                        return Err(crate::error::Error::InvalidArgs(format!(
                            "context mismatch at line {}",
                            idx + 1
                        )));
                    }
                }
                applied += 1;
                i += 1;
            }
            let _ = (start, applied);
        } else {
            i += 1;
        }
    }
    Ok(lines.join("\n"))
}

/// Parse the new-file start line number from a hunk header.
fn parse_hunk_start(header: &str) -> Result<usize> {
    // header looks like "-a,b +c,d @@"
    let plus = header
        .find('+')
        .ok_or_else(|| crate::error::Error::InvalidArgs("bad hunk header".into()))?;
    let rest = &header[plus + 1..];
    let end = rest
        .find(' ')
        .or_else(|| rest.find(','))
        .unwrap_or(rest.len());
    rest[..end]
        .parse::<usize>()
        .map_err(|_| crate::error::Error::InvalidArgs("bad hunk start".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_a_simple_patch() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello world").unwrap();
        let patch = "--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-hello world\n+hello forge\n";
        apply_unified_diff(dir.path(), patch).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
            "hello forge"
        );
    }

    #[test]
    fn creates_a_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let patch = "--- a/new.txt\n+++ b/new.txt\n@@ -0,0 +1 @@\n+content\n";
        apply_unified_diff(dir.path(), patch).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("new.txt")).unwrap(),
            "content"
        );
    }
}
