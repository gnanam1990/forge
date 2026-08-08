//! Git operation tools: stash, tag, remote, reset, checkout, merge, push,
//! pull, fetch, and add. Each runs a git command in the workspace and returns
//! its output.

use std::process::Command;

use serde_json::Value;

use super::{optional_string_arg, string_arg, Tool, ToolContext, ToolResult};
use crate::error::Result;

/// Run a git command in the workspace and return its output.
fn git(ctx: &ToolContext, args: &[&str]) -> Result<ToolResult> {
    let output = Command::new("git")
        .args(args)
        .current_dir(&ctx.workspace_root)
        .output()?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    if output.status.success() {
        Ok(ToolResult::ok(text))
    } else {
        Ok(ToolResult::err(format!(
            "git exited with {}\n{text}",
            output.status
        )))
    }
}

/// `git_stash` — stash operations.
#[derive(Default)]
pub struct GitStashTool;

impl GitStashTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitStashTool {
    fn name(&self) -> &str {
        "git_stash"
    }
    fn description(&self) -> &str {
        "Stash operations. Args: {\"action\": \"push|pop|list|apply|drop\", \"index\": number (optional)}."
    }
    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let action = optional_string_arg(args, "action").unwrap_or_else(|| "list".into());
        let index = args.get("index").and_then(Value::as_u64);
        let mut cmd: Vec<String> = vec!["stash".into()];
        match action.as_str() {
            "push" => cmd.push("push".into()),
            "pop" => cmd.push("pop".into()),
            "apply" => {
                cmd.push("apply".into());
                if let Some(i) = index {
                    cmd.push(format!("stash@{{{i}}}"));
                }
            }
            "drop" => {
                cmd.push("drop".into());
                if let Some(i) = index {
                    cmd.push(format!("stash@{{{i}}}"));
                }
            }
            _ => cmd.push("list".into()),
        }
        let refs: Vec<&str> = cmd.iter().map(String::as_str).collect();
        git(ctx, &refs)
    }
}

/// `git_tag` — tag operations.
#[derive(Default)]
pub struct GitTagTool;

impl GitTagTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitTagTool {
    fn name(&self) -> &str {
        "git_tag"
    }
    fn description(&self) -> &str {
        "Tag operations. Args: {\"action\": \"list|create|delete\", \"name\": string, \"message\": string (optional)}."
    }
    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let action = optional_string_arg(args, "action").unwrap_or_else(|| "list".into());
        let name = optional_string_arg(args, "name");
        let message = optional_string_arg(args, "message");
        let mut cmd: Vec<String> = vec!["tag".into()];
        match action.as_str() {
            "create" => {
                cmd.push("-a".into());
                if let Some(name) = name {
                    cmd.push(name);
                }
                if let Some(msg) = message {
                    cmd.push("-m".into());
                    cmd.push(msg);
                }
            }
            "delete" => {
                cmd.push("-d".into());
                if let Some(name) = name {
                    cmd.push(name);
                }
            }
            _ => cmd.push("-l".into()),
        }
        let refs: Vec<&str> = cmd.iter().map(String::as_str).collect();
        git(ctx, &refs)
    }
}

/// `git_remote` — remote operations.
#[derive(Default)]
pub struct GitRemoteTool;

impl GitRemoteTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitRemoteTool {
    fn name(&self) -> &str {
        "git_remote"
    }
    fn description(&self) -> &str {
        "Remote operations. Args: {\"action\": \"list|add|remove|rename|set-url\", \"name\": string, \"url\": string, \"new\": string (optional)}."
    }
    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let action = optional_string_arg(args, "action").unwrap_or_else(|| "list".into());
        let name = optional_string_arg(args, "name");
        let url = optional_string_arg(args, "url");
        let new = optional_string_arg(args, "new");
        let mut cmd: Vec<String> = vec!["remote".into()];
        match action.as_str() {
            "add" => {
                cmd.push("add".into());
                if let Some(name) = name {
                    cmd.push(name);
                }
                if let Some(url) = url {
                    cmd.push(url);
                }
            }
            "remove" => {
                cmd.push("remove".into());
                if let Some(name) = name {
                    cmd.push(name);
                }
            }
            "rename" => {
                cmd.push("rename".into());
                if let Some(name) = name {
                    cmd.push(name);
                }
                if let Some(new) = new {
                    cmd.push(new);
                }
            }
            "set-url" => {
                cmd.push("set-url".into());
                if let Some(name) = name {
                    cmd.push(name);
                }
                if let Some(url) = url {
                    cmd.push(url);
                }
            }
            _ => cmd.push("-v".into()),
        }
        let refs: Vec<&str> = cmd.iter().map(String::as_str).collect();
        git(ctx, &refs)
    }
}

/// `git_reset` — reset the working tree.
#[derive(Default)]
pub struct GitResetTool;

impl GitResetTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitResetTool {
    fn name(&self) -> &str {
        "git_reset"
    }
    fn description(&self) -> &str {
        "Reset the working tree. Args: {\"mode\": \"soft|hard|mixed\", \"commit\": string (optional)}."
    }
    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let mode = optional_string_arg(args, "mode").unwrap_or_else(|| "mixed".into());
        let commit = optional_string_arg(args, "commit");
        let mut cmd: Vec<String> = vec!["reset".into()];
        match mode.as_str() {
            "soft" => cmd.push("--soft".into()),
            "hard" => cmd.push("--hard".into()),
            _ => cmd.push("--mixed".into()),
        }
        if let Some(commit) = commit {
            cmd.push(commit);
        }
        let refs: Vec<&str> = cmd.iter().map(String::as_str).collect();
        git(ctx, &refs)
    }
}

/// `git_checkout` — checkout a branch or commit.
#[derive(Default)]
pub struct GitCheckoutTool;

impl GitCheckoutTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitCheckoutTool {
    fn name(&self) -> &str {
        "git_checkout"
    }
    fn description(&self) -> &str {
        "Checkout a branch or commit. Args: {\"reference\": string, \"branch\": bool (optional)}."
    }
    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let reference = string_arg(args, "reference")?;
        let branch = args.get("branch").and_then(Value::as_bool).unwrap_or(false);
        let mut cmd: Vec<String> = vec!["checkout".into()];
        if branch {
            cmd.push("-b".into());
        }
        cmd.push(reference);
        let refs: Vec<&str> = cmd.iter().map(String::as_str).collect();
        git(ctx, &refs)
    }
}

/// `git_merge` — merge a branch.
#[derive(Default)]
pub struct GitMergeTool;

impl GitMergeTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitMergeTool {
    fn name(&self) -> &str {
        "git_merge"
    }
    fn description(&self) -> &str {
        "Merge a branch into the current branch. Args: {\"branch\": string}."
    }
    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let branch = string_arg(args, "branch")?;
        git(ctx, &["merge", &branch])
    }
}

/// `git_push` — push commits.
#[derive(Default)]
pub struct GitPushTool;

impl GitPushTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitPushTool {
    fn name(&self) -> &str {
        "git_push"
    }
    fn description(&self) -> &str {
        "Push commits to a remote. Args: {\"remote\": string (optional), \"branch\": string (optional), \"force\": bool (optional)}."
    }
    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let remote = optional_string_arg(args, "remote");
        let branch = optional_string_arg(args, "branch");
        let force = args.get("force").and_then(Value::as_bool).unwrap_or(false);
        let mut cmd: Vec<String> = vec!["push".into()];
        if force {
            cmd.push("--force".into());
        }
        if branch.is_some() && remote.is_none() {
            cmd.push("origin".into());
        }
        if let Some(remote) = remote {
            cmd.push(remote);
        }
        if let Some(branch) = branch {
            cmd.push(branch);
        }
        let refs: Vec<&str> = cmd.iter().map(String::as_str).collect();
        git(ctx, &refs)
    }
}

/// `git_pull` — pull changes.
#[derive(Default)]
pub struct GitPullTool;

impl GitPullTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitPullTool {
    fn name(&self) -> &str {
        "git_pull"
    }
    fn description(&self) -> &str {
        "Pull changes from the remote. Args: {}."
    }
    fn run(&self, _args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        git(ctx, &["pull"])
    }
}

/// `git_fetch` — fetch changes.
#[derive(Default)]
pub struct GitFetchTool;

impl GitFetchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitFetchTool {
    fn name(&self) -> &str {
        "git_fetch"
    }
    fn description(&self) -> &str {
        "Fetch changes from the remote. Args: {}."
    }
    fn run(&self, _args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        git(ctx, &["fetch"])
    }
}

/// `git_add` — stage files.
#[derive(Default)]
pub struct GitAddTool;

impl GitAddTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for GitAddTool {
    fn name(&self) -> &str {
        "git_add"
    }
    fn description(&self) -> &str {
        "Stage files. Args: {\"files\": [string]}."
    }
    fn run(&self, args: &Value, ctx: &ToolContext) -> Result<ToolResult> {
        let files = args
            .get("files")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();
        if files.is_empty() {
            return Ok(ToolResult::err(
                "`files` must be a non-empty array".to_string(),
            ));
        }
        let refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let mut cmd: Vec<&str> = vec!["add"];
        cmd.extend(refs);
        git(ctx, &cmd)
    }
}
