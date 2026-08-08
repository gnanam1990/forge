//! Command-line interface for forge.

use std::io::Write;
use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};

use crate::agent::{http::HttpProvider, Agent};
use crate::config::Config;
use crate::error::Result;
use crate::tools::Registry;

/// forge — an original, self-contained coding agent written in Rust.
#[derive(Parser)]
#[command(name = "forge", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run the agent on a prompt.
    Run {
        /// The user prompt.
        prompt: String,
        /// Read the prompt from a file instead of the argument.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Resume a saved session by id.
        #[arg(long)]
        resume: Option<String>,
        /// Send a completion notification.
        #[arg(long)]
        notify: bool,
        /// Override the workspace root.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Override the max number of turns.
        #[arg(long)]
        max_turns: Option<usize>,
        /// Re-run the agent whenever files in the workspace change.
        #[arg(long)]
        watch: bool,
    },
    /// Print the version.
    Version,
    /// List the built-in model catalog.
    Models,
    /// Show or set the provider: `provider [show|set <model>]`.
    Provider {
        #[command(subcommand)]
        action: ProviderAction,
    },
    /// Print a short usage guide.
    Docs,
    /// Show usage stats from telemetry.
    Stats,
    /// Show environment information.
    Env {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Start an interactive shell session.
    Shell,
    /// Watch a directory and run a command on change: `watch <dir> <command>`.
    Watch {
        /// The directory to watch.
        dir: PathBuf,
        /// The command to run on change.
        command: String,
    },
    /// Run a simple benchmark.
    Benchmark,
    /// Print a summary of all commands.
    Help,
    /// List the available tools.
    Tools,
    /// Run several prompts as parallel sub-agents. The file is a JSON array of
    /// prompt strings.
    Orchestrate {
        /// Path to a JSON file containing an array of prompt strings.
        file: PathBuf,
        /// Override the max number of turns per sub-agent.
        #[arg(long)]
        max_turns: Option<usize>,
    },
    /// Run a workflow from a JSON file (a DAG of tasks).
    Workflow {
        /// Path to a JSON workflow file.
        file: PathBuf,
        /// Override the max number of turns per task.
        #[arg(long)]
        max_turns: Option<usize>,
        /// Send a completion notification.
        #[arg(long)]
        notify: bool,
    },
    /// Resume a saved session with a new prompt.
    Resume {
        /// The session id to resume.
        session: String,
        /// The new prompt.
        prompt: String,
        /// Override the max number of turns.
        #[arg(long)]
        max_turns: Option<usize>,
    },
    /// List saved sessions.
    Sessions,
    /// Export a session to a JSON file: `export <id> <path>`.
    Export {
        /// The session id.
        id: String,
        /// The output file path.
        path: PathBuf,
    },
    /// Import a session from a JSON file: `import <path>`.
    Import {
        /// The input file path.
        path: PathBuf,
    },
    /// Start an interactive chat session.
    Chat {
        /// Resume a saved session by id.
        #[arg(long)]
        resume: Option<String>,
    },
    /// Manage cross-session memory: `remember <key> <value>`, `recall <key>`,
    /// `list`, `clear`, `export <path>`.
    Memory {
        /// Subcommand: remember | recall | list.
        action: String,
        /// Key (for remember/recall).
        key: Option<String>,
        /// Value (for remember).
        value: Option<String>,
    },
    /// Run scheduled jobs from a JSON file. With `--forever`, keep running.
    Cron {
        /// Path to a JSON file containing an array of jobs.
        file: PathBuf,
        /// Keep running on the jobs' intervals instead of once.
        #[arg(long)]
        forever: bool,
    },
    /// Review the current git diff for common issues.
    Review,
    /// Interact with MCP servers: `mcp call <server> <tool> <args>`,
    /// `mcp add <name> <command> [args...]`, `mcp list`, `mcp remove <name>`.
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
    /// Show the current git diff.
    Diff {
        /// Show only staged changes.
        #[arg(long)]
        staged: bool,
        /// Show diff statistics instead of the full diff.
        #[arg(long)]
        stat: bool,
        /// List only the names of changed files.
        #[arg(long)]
        name_only: bool,
    },
    /// Commit staged changes with an AI-generated or explicit message.
    Commit {
        /// Use this message instead of generating one.
        message: Option<String>,
        /// Stage all changes before committing.
        #[arg(long)]
        all: bool,
        /// Amend the last commit instead of creating a new one.
        #[arg(long)]
        amend: bool,
    },
    /// Run the project's build command.
    Build,
    /// Run the project's test command.
    Test,
    /// Run the project's lint command.
    Lint,
    /// Run the project's formatter.
    Format,
    /// Type-check the project.
    Check,
    /// Spawn a standalone sub-agent on a prompt: `agent <prompt>`.
    Agent {
        /// The prompt for the sub-agent.
        prompt: String,
        /// Override the max number of turns.
        #[arg(long)]
        max_turns: Option<usize>,
    },
    /// Audit project dependencies.
    Audit,
    /// Scan the codebase for TODO/FIXME/HACK markers.
    Todo,
    /// Launch a headless browser and open a URL.
    Browser {
        /// The URL to open.
        url: String,
        /// Evaluate a JavaScript expression in the opened page.
        #[arg(long)]
        eval: Option<String>,
        /// Click at a coordinate: `--click x,y`.
        #[arg(long)]
        click: Option<String>,
        /// Type text into the page.
        #[arg(long)]
        r#type: Option<String>,
        /// Save a screenshot to a path.
        #[arg(long)]
        screenshot: Option<String>,
        /// Wait this many milliseconds before extracting text or screenshots.
        #[arg(long)]
        wait: Option<u64>,
        /// Print the visible text of the page.
        #[arg(long)]
        text: bool,
    },
    /// Desktop control: `screenshot <path>`, `click <x> <y>`, `type <text>`.
    Desktop {
        /// Action: screenshot | click | type.
        action: String,
        /// Arguments for the action.
        args: Vec<String>,
    },
    /// Write a working config plus sample workflow and cron files.
    Setup,
    /// Check the environment and report what is available.
    Doctor {
        /// Attempt to fix config issues.
        #[arg(long)]
        fix: bool,
        /// Emit the checks as JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Print version, tools, and config summary.
    Info,
    /// Manage the config: `config` (show), `config reset`, `config get <key>`,
    /// `config set <key> <value>`.
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// Manage sessions: `session <delete|rename> <id> [new]`.
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Check for updates against the remote.
    Update,
    /// View the telemetry log.
    Log,
    /// Manage telemetry: `telemetry on`, `telemetry off`, `telemetry export <path>`.
    Telemetry {
        #[command(subcommand)]
        action: TelemetryAction,
    },
    /// Manage command aliases: `alias` (list), `alias <name> <command>` (add),
    /// `alias remove <name>` (remove).
    Alias {
        /// The alias name (omit to list all aliases).
        name: Option<String>,
        /// The command it expands to (omit to remove the alias).
        command: Option<String>,
    },
    /// Manage plugins: `plugin <list|enable|disable|add>`.
    Plugin {
        /// Action: list | enable | disable | add.
        action: String,
        /// Plugin name (for enable/disable) or file path (for add).
        arg: Option<String>,
    },
    /// List plugins loaded from the configured plugins directory.
    Plugins,
    /// Set the session effort posture: `effort <auto|balanced|thorough|zeromaxing>`.
    Effort {
        /// The effort level.
        level: String,
    },
    /// Run a typed plan from a JSON file (the zeromaxing plan model).
    Plan {
        /// Path to a JSON plan file.
        file: PathBuf,
    },
    /// Start the HTTP API server: `serve [--bind 127.0.0.1:8787]`.
    Serve {
        /// The address to bind, e.g. `127.0.0.1:8787`.
        #[arg(long, default_value = "127.0.0.1:8787")]
        bind: String,
    },
    /// Back up sessions, config, and memory to a single JSON file.
    Backup {
        /// The output file path.
        path: PathBuf,
    },
    /// Restore sessions, config, and memory from a backup file.
    Restore {
        /// The backup file path.
        path: PathBuf,
    },
    /// Count tokens in a prompt or file: `token <text>` or `token --file <path>`.
    Token {
        /// The text to count.
        text: Option<String>,
        /// Read the text from a file instead of the argument.
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// List hooks configured in the config.
    Hooks,
    /// Show the permission policy for tools.
    Permission,
    /// Run git subcommands: `git <log|branch|branch-create|switch|stash list|blame|tag|remote|status>`.
    Git {
        #[command(subcommand)]
        action: GitAction,
    },
    /// Create a pull request with the `gh` CLI: `pr [--base main] [--title ...]`.
    Pr {
        /// The base branch to target.
        #[arg(long, default_value = "main")]
        base: String,
        /// The pull request title (else generated from the last commit).
        #[arg(long)]
        title: Option<String>,
        /// The pull request body (else a short default).
        #[arg(long)]
        body: Option<String>,
    },
    /// Run a command inside the sandbox: `sandbox run <command>`.
    Sandbox {
        #[command(subcommand)]
        action: SandboxAction,
    },
    /// Search the codebase index for files containing a query.
    Search {
        /// The search query.
        query: String,
    },
    /// Search the web: `web <query>`.
    Web {
        /// The web search query.
        query: String,
    },
    /// Show a session's token/context usage: `context <session-id>`.
    Context {
        /// The session id.
        session: String,
    },
    /// Switch the active model: `model use <index|name>`.
    Model {
        #[command(subcommand)]
        action: ModelAction,
    },
    /// Write a sample config file to the default location, or scaffold a
    /// project from a template: `init --template <rust|node|python|web>`.
    Init {
        /// The project template to scaffold.
        #[arg(long)]
        template: Option<String>,
    },
    /// Start an interactive REPL that runs prompts in a persistent session.
    Repl,
    /// Run a command on a remote host over SSH: `ssh <host> <command>`.
    Ssh {
        /// The remote host.
        host: String,
        /// The command to run.
        command: String,
    },
    /// Run a terminal command in the workspace: `terminal <command>`.
    Terminal {
        /// The command to run.
        command: String,
    },
    /// Generate shell completions: `completions <bash|zsh|fish>`.
    Completions {
        /// The shell to generate completions for.
        shell: String,
    },
    /// Print a man-style reference for forge.
    Man,
    /// Self-update by pulling the repo and rebuilding.
    Upgrade,
    /// Show a summary of the current identity and configuration.
    Whoami,
}

/// Subcommands for `forge config`.
#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show the current config (with the API key redacted).
    Show,
    /// Reset the config to defaults.
    Reset,
    /// Read a config value.
    Get {
        /// The config key (e.g. provider.model, max_turns).
        key: String,
    },
    /// Set a config value.
    Set {
        /// The config key (e.g. provider.model, max_turns).
        key: String,
        /// The value.
        value: String,
    },
    /// Validate the config file and report any problems.
    Validate,
    /// Print the path to the config file.
    Path,
    /// Remove a config key.
    Unset {
        /// The config key (e.g. provider.model, max_turns).
        key: String,
    },
    /// Show the resolved build/test/lint/format/check commands for the workspace.
    Commands,
    /// List all effective config keys and values.
    List,
}

/// Subcommands for `forge telemetry`.
#[derive(Subcommand)]
pub enum TelemetryAction {
    /// Enable telemetry.
    On,
    /// Disable telemetry.
    Off,
    /// Export the telemetry log to a file.
    Export {
        /// The output file path.
        path: PathBuf,
    },
    /// Clear the telemetry log.
    Clear,
    /// Show aggregate usage stats from the telemetry log.
    Stats,
    /// Print the path to the telemetry log.
    Path,
}

/// Subcommands for `forge model`.
#[derive(Subcommand)]
pub enum ModelAction {
    /// Switch the active model to a saved provider.
    Use {
        /// The saved provider index or model name.
        target: String,
    },
    /// List saved providers.
    List,
}

/// Subcommands for `forge provider`.
#[derive(Subcommand)]
pub enum ProviderAction {
    /// Show the active provider.
    Show,
    /// Set the active model.
    Set {
        /// The model to set.
        model: String,
    },
    /// List saved providers.
    List,
    /// Add a saved provider.
    Add {
        /// Optional provider name.
        name: Option<String>,
        /// The model.
        model: String,
        /// Optional base URL.
        base_url: Option<String>,
    },
    /// Remove a saved provider by index or model name.
    Remove {
        /// The provider index or model name.
        target: String,
    },
}

/// Subcommands for `forge session`.
#[derive(Subcommand)]
pub enum SessionAction {
    /// List saved sessions with message and token counts.
    List,
    /// Show details for a session.
    Show {
        /// The session id.
        id: String,
    },
    /// Delete a session.
    Delete {
        /// The session id.
        id: String,
    },
    /// Rename a session.
    Rename {
        /// The session id.
        id: String,
        /// The new id.
        new: String,
    },
    /// Export a session to a JSON file.
    Export {
        /// The session id.
        id: String,
        /// The output file path.
        path: PathBuf,
    },
    /// Import a session from a JSON file.
    Import {
        /// The input file path.
        path: PathBuf,
    },
}

/// Subcommands for `forge git stash`.
#[derive(Subcommand)]
pub enum StashAction {
    /// List stashed changes.
    List,
    /// Stash uncommitted changes.
    Push,
    /// Restore the most recent stash.
    Pop,
    /// Apply a specific stash by index.
    Apply {
        /// The stash index (default 0).
        index: Option<usize>,
    },
}

/// Subcommands for `forge git tag`.
#[derive(Subcommand)]
pub enum TagAction {
    /// List tags.
    List,
    /// Create an annotated tag.
    Create {
        /// The tag name.
        name: String,
        /// The tag message.
        message: Option<String>,
    },
    /// Delete a tag.
    Delete {
        /// The tag name.
        name: String,
    },
}

/// Subcommands for `forge git remote`.
#[derive(Subcommand)]
pub enum RemoteAction {
    /// Show the configured remotes.
    Show,
    /// Add a remote.
    Add {
        /// The remote name.
        name: String,
        /// The remote URL.
        url: String,
    },
    /// Remove a remote.
    Remove {
        /// The remote name.
        name: String,
    },
    /// Rename a remote.
    Rename {
        /// The old remote name.
        old: String,
        /// The new remote name.
        new: String,
    },
}

/// Subcommands for `forge git`.
#[derive(Subcommand)]
pub enum GitAction {
    /// Show recent commit history.
    Log {
        /// Limit the number of commits shown.
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Show diff statistics for each commit.
        #[arg(long)]
        stat: bool,
        /// Filter commits by author.
        #[arg(long)]
        author: Option<String>,
        /// Filter commits whose message matches a pattern.
        #[arg(long)]
        grep: Option<String>,
        /// Show only commits after a date (e.g. "2024-01-01", "2 weeks ago").
        #[arg(long)]
        since: Option<String>,
        /// Include commits from all branches.
        #[arg(long)]
        all: bool,
    },
    /// List branches.
    Branch,
    /// Create and switch to a new branch: `git branch-create <name>`.
    BranchCreate {
        /// The new branch name.
        name: String,
    },
    /// Switch to an existing branch: `git switch <name>`.
    Switch {
        /// The branch name.
        name: String,
    },
    /// Manage the stash: `stash` (help), `stash list`, `stash push`, `stash pop`.
    Stash {
        #[command(subcommand)]
        action: StashAction,
    },
    /// Show who last changed each line of a file: `git blame <file>`.
    Blame {
        /// The file path.
        file: String,
    },
    /// Manage tags: `tag` (help), `tag list`, `tag create <name> [msg]`, `tag delete <name>`.
    Tag {
        #[command(subcommand)]
        action: TagAction,
    },
    /// Manage remotes: `remote` (show), `remote add <name> <url>`, `remote remove <name>`.
    Remote {
        #[command(subcommand)]
        action: RemoteAction,
    },
    /// Show the working tree status.
    Status,
    /// Stage files: `git add <file>...`.
    Add {
        /// The files to stage.
        files: Vec<String>,
    },
    /// Commit staged changes: `git commit [-m <msg>] [--all]`.
    Commit {
        /// The commit message.
        message: Option<String>,
        /// Stage all changes before committing.
        #[arg(long)]
        all: bool,
    },
    /// Push commits to a remote: `git push [--remote <name>] [--branch <name>] [--force]`.
    Push {
        /// The remote name (default: origin).
        #[arg(long)]
        remote: Option<String>,
        /// The branch to push (default: current branch).
        #[arg(long)]
        branch: Option<String>,
        /// Force the push.
        #[arg(long)]
        force: bool,
    },
    /// Pull changes from the remote.
    Pull,
    /// Fetch changes from the remote.
    Fetch,
    /// Reset the working tree: `git reset [--soft|--hard] [<commit>]`.
    Reset {
        /// Soft reset (keep changes staged).
        #[arg(long)]
        soft: bool,
        /// Hard reset (discard changes).
        #[arg(long)]
        hard: bool,
        /// The commit to reset to (default: HEAD).
        commit: Option<String>,
    },
    /// Show a commit: `git show <ref>`.
    Show {
        /// The commit reference.
        reference: String,
        /// Show diff statistics.
        #[arg(long)]
        stat: bool,
    },
    /// Merge a branch into the current branch: `git merge <branch>`.
    Merge {
        /// The branch to merge.
        branch: String,
    },
    /// Checkout a branch or commit: `git checkout [--branch] <ref>`.
    Checkout {
        /// Create the branch before checking it out.
        #[arg(long)]
        branch: bool,
        /// The branch or commit to check out.
        reference: String,
    },
    /// Remove untracked files: `git clean [-f]`.
    Clean {
        /// Force removal (required by git).
        #[arg(long)]
        force: bool,
    },
    /// Show the diff: `git diff [--staged] [--stat] [--name-only]`.
    Diff {
        /// Show only staged changes.
        #[arg(long)]
        staged: bool,
        /// Show diff statistics instead of the full diff.
        #[arg(long)]
        stat: bool,
        /// List only the names of changed files.
        #[arg(long)]
        name_only: bool,
    },
    /// Cherry-pick a commit onto the current branch: `git cherry-pick <commit>`.
    CherryPick {
        /// The commit to apply.
        commit: String,
    },
    /// Rebase the current branch onto another branch: `git rebase <branch>`.
    Rebase {
        /// The branch to rebase onto.
        branch: String,
    },
}

/// Subcommands for `forge sandbox`.
#[derive(Subcommand)]
pub enum SandboxAction {
    /// Run a command inside the sandbox.
    Run {
        /// The command to run.
        command: String,
    },
}

/// Subcommands for `forge mcp`.
#[derive(Subcommand)]
pub enum McpAction {
    /// Call a tool on an MCP server: `mcp call <server> <tool> <json-args>`.
    Call {
        /// The MCP server command to spawn.
        server: String,
        /// The tool to call.
        tool: String,
        /// JSON arguments for the tool.
        args: String,
    },
    /// Register an MCP server in the config: `mcp add <name> <command> [args...]`.
    Add {
        /// The server name.
        name: String,
        /// The command to spawn.
        command: String,
        /// Extra arguments for the command.
        args: Vec<String>,
    },
    /// List MCP servers registered in the config.
    List,
    /// Remove an MCP server from the config: `mcp remove <name>`.
    Remove {
        /// The server name.
        name: String,
    },
    /// Test a connection to an MCP server and list its tools.
    Test {
        /// The server name.
        name: String,
    },
}

/// Run the CLI and return a process exit code.
pub fn run(cli: Cli) -> i32 {
    match dispatch(cli) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            exit_code(&e)
        }
    }
}

/// Map an error to a distinct exit code.
fn exit_code(e: &crate::error::Error) -> i32 {
    use crate::error::Error;
    match e {
        Error::Config(_) => 2,
        Error::Provider(_) => 3,
        Error::Tool(_) => 4,
        Error::InvalidArgs(_) => 5,
        Error::Agent(_) => 6,
        _ => 1,
    }
}

/// Check whether a binary is on PATH.
fn which(name: &str) -> bool {
    let path = std::env::var("PATH").unwrap_or_default();
    path.split(':')
        .any(|dir| std::path::Path::new(dir).join(name).exists())
}

/// Generate a conventional commit message from a staged diff using the
/// configured provider. Falls back to a generic message if the provider is
/// unavailable.
fn generate_commit_message(config: &Config, diff: &str) -> Result<String> {
    use crate::agent::{Message, Provider};
    let provider = crate::agent::http::HttpProvider::new(&config.provider)?;
    let prompt = format!(
        "Write a concise conventional commit message (type: subject) for the \
         following diff. Use one line, imperative mood, under 72 characters. \
         Do not include any other text.\n\n{diff}"
    );
    let reply = provider.complete(&[Message::User(prompt)])?;
    let msg = reply.content.trim().to_string();
    if msg.is_empty() {
        Ok("chore: update".to_string())
    } else {
        Ok(msg)
    }
}

/// Run a project command (build/test/lint) in the workspace and stream its
/// output. Returns an error if the command exits non-zero.
fn run_project_command(config: &Config, step: &str) -> Result<()> {
    let wiring = crate::wiring::build_wiring(config)?;
    wiring
        .telemetry
        .record("project", serde_json::json!({ "step": step }))?;
    let ws = config.workspace_root();
    let cmd = config.commands.resolve(step, &ws);
    eprintln!("[forge] {step}: {cmd}");
    let status = std::process::Command::new("sh")
        .args(["-c", &cmd])
        .current_dir(&ws)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(crate::error::Error::Tool(format!(
            "{step} failed with {status}"
        )))
    }
}

/// Scan a workspace for TODO/FIXME/HACK markers and print them grouped by file.
fn scan_todos(root: &std::path::Path) -> Result<()> {
    use walkdir::WalkDir;
    let re = regex::Regex::new(r"(?i)\b(TODO|FIXME|HACK)\b").unwrap();
    let mut found = 0;
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if path.components().any(|c| {
            let name = c.as_os_str().to_string_lossy();
            name == "target" || name == ".git" || name == "node_modules" || name == ".forge"
        }) {
            continue;
        }
        if let Ok(raw) = std::fs::read_to_string(path) {
            for (idx, line) in raw.lines().enumerate() {
                if line.chars().count() > 500 {
                    continue; // skip minified/generated single-line files
                }
                if re.is_match(line) {
                    let rel = path.strip_prefix(root).unwrap_or(path);
                    println!("{}:{}: {}", rel.display(), idx + 1, line.trim());
                    found += 1;
                }
            }
        }
    }
    if found == 0 {
        println!("no TODO/FIXME/HACK markers found");
    }
    Ok(())
}

/// Scaffold a project from a named template in the current directory.
fn scaffold_template(name: &str) -> Result<()> {
    use std::fs;
    let cwd = std::env::current_dir()?;
    let write = |rel: &str, content: &str| -> Result<()> {
        let path = cwd.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
        println!("wrote {}", path.display());
        Ok(())
    };
    match name {
        "rust" => {
            write("Cargo.toml", "[package]\nname = \"my_project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n")?;
            write(
                "src/main.rs",
                "fn main() {\n    println!(\"hello from rust\");\n}\n",
            )?;
            write("README.md", "# My Rust Project\n")?;
        }
        "node" => {
            write(
                "package.json",
                "{\n  \"name\": \"my-project\",\n  \"version\": \"0.1.0\",\n  \"main\": \"index.js\",\n  \"scripts\": {\n    \"start\": \"node index.js\"\n  }\n}\n",
            )?;
            write("index.js", "console.log('hello from node');\n")?;
            write("README.md", "# My Node Project\n")?;
        }
        "python" => {
            write(
                "pyproject.toml",
                "[project]\nname = \"my-project\"\nversion = \"0.1.0\"\n",
            )?;
            write("main.py", "def main():\n    print('hello from python')\n\nif __name__ == '__main__':\n    main()\n")?;
            write("README.md", "# My Python Project\n")?;
        }
        "web" => {
            write("index.html", "<!doctype html>\n<html>\n<head><title>My Site</title></head>\n<body><h1>Hello from web</h1></body>\n</html>\n")?;
            write("style.css", "body { font-family: sans-serif; }\n")?;
            write("README.md", "# My Web Project\n")?;
        }
        other => {
            return Err(crate::error::Error::InvalidArgs(format!(
                "unknown template {other}; use rust, node, python, or web"
            )))
        }
    }
    println!("scaffolded {name} project in {}", cwd.display());
    Ok(())
}

/// Run an interactive line-based REPL that keeps a persistent session.
fn run_repl(config: &Config) -> Result<()> {
    use std::io::{BufRead, Write};
    let turns = config.max_turns.unwrap_or(10);
    let provider = HttpProvider::new(&config.provider)?;
    let wiring = crate::wiring::build_wiring(config)?;
    let agent = Agent::new(Box::new(provider), wiring.registry, turns).with_hooks(wiring.hooks);
    let dir = crate::session::default_sessions_dir()?;
    let id = format!(
        "repl-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    let mut session = crate::session::Session::new(&id);
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    println!("forge repl — type a prompt, or `exit` to quit. Session: {id}");
    loop {
        print!("> ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let prompt = line.trim().to_string();
        if prompt.is_empty() {
            continue;
        }
        if prompt == "exit" || prompt == "quit" {
            break;
        }
        match agent.run_into(&mut session.messages, &prompt) {
            Ok(outcome) => {
                println!("{}", outcome.final_text);
                eprintln!(
                    "[forge] {} turn(s), {} tool call(s)",
                    outcome.turns, outcome.tool_calls
                );
            }
            Err(e) => eprintln!("error: {e}"),
        }
    }
    session.save(&dir)?;
    println!("session saved as {id}");
    Ok(())
}

/// Print a summary of all commands.
fn print_help() {
    println!(
        "forge — a coding agent in Rust\n\n\
         run <prompt> [--file] [--workspace] [--max-turns]   run the agent\n\
         chat                                          interactive TUI\n\
         resume <id> <prompt>                           resume a session\n\
         sessions                                      list sessions\n\
         export <id> <path> / import <path>            session export/import\n\
         orchestrate <file>                            parallel sub-agents\n\
         workflow <file>                                run a workflow\n\
         plan <file>                                    run a typed plan\n\
         effort <level>                                set effort posture\n\
         tools                                         list tools\n\
         models                                        list models\n\
         memory <remember|recall|search|list|stats|remove|clear|export|import>  manage memory\n\
         cron <file> [--forever]                       run scheduled jobs\n\
         review                                        review the git diff\n\
         mcp call <server> <tool> <args>               call an MCP tool\n\
         mcp add/list/remove/test                      manage MCP servers\n\
         diff [--staged]                                show the git diff\n\
         commit [--all] [message]                      commit changes\n\
         build / test / lint / format / check          run project commands\n\
         agent <prompt>                                spawn a sub-agent\n\
         audit                                         audit dependencies\n\
         todo                                          scan for TODO/FIXME\n\
         serve [--bind ADDR]                           start the HTTP API\n\
         backup <path> / restore <path>                 backup/restore state\n\
         token <text> [--file]                         count tokens\n\
         hooks                                         list hooks\n\
         permission                                    show tool permissions\n\
         git <log|diff|add|commit|push|pull|fetch|reset|show|merge|checkout|rebase|cherry-pick|branch|stash list|remote|status|blame|tag|switch>  git helpers\n\
         web <query>                                       web search\n\
         pr [--base main] [--title]                    create a pull request\n\
         sandbox run <cmd>                              run a command in the sandbox\n\
         search <query>                                search the codebase\n\
         context <session-id>                          show session context usage\n\
         model use <index|name>                        switch the active model\n\
         browser <url> [--eval] [--click] [--type] [--screenshot]  browser\n\
         desktop <screenshot|click|type>               desktop control\n\
         plugin <list|enable|disable|add>              manage plugins\n\
         alias [name] [command]                        list/add/remove aliases\n\
         config [show|reset|get <key>|set <key> <value>]  manage config\n\
         doctor [--json] [--fix]                       check environment\n\
         info                                          show summary\n\
         setup                                         write config + samples\n\
         init [--template <rust|node|python|web>]      scaffold a project\n\
         repl                                          interactive REPL\n\
         ssh <host> <command>                          run a command over SSH\n\
         terminal <command>                             run a terminal command\n\
         config get <key>                              read a config value\n\
         completions <bash|zsh|fish>                   shell completions\n\
         man                                           man-style reference\n\
         upgrade                                       self-update and rebuild\n\
         whoami                                        show identity summary\n\
         version                                       print version\n\
         help                                          this help"
    );
}

/// Print a man-style reference for forge.
fn print_man() {
    println!(
        "NAME\n    forge — a coding agent in Rust\n\n\
         SYNOPSIS\n    forge <command> [options]\n\n\
         DESCRIPTION\n    forge is a self-contained coding agent. It runs a model\n\
         provider in a loop, calling tools (read/write/edit files, run bash,\n\
         search, git, web, browser, desktop) until the task is done.\n\n\
         COMMANDS\n    run <prompt>            run the agent on a prompt\n\
         chat                    interactive TUI\n\
         resume <id> <prompt>    resume a session\n\
         sessions                list sessions\n\
         export/import           session export/import\n\
         orchestrate <file>      parallel sub-agents\n\
         workflow <file>          run a workflow DAG\n\
         plan <file>              run a typed plan\n\
         memory                  manage cross-session memory\n\
         cron <file>              run scheduled jobs\n\
         review                  review the git diff\n\
         mcp                     call/manage MCP servers\n\
         browser / desktop       browser and desktop control\n\
         plugin                  manage plugins\n\
         alias                   manage command aliases\n\
         config                  show or reset config\n\
         doctor                  check the environment\n\
         diff / commit           git diff and commit\n\
         build/test/lint/format/check  run project commands\n\
         agent <prompt>          spawn a sub-agent\n\
         audit                   audit dependencies\n\
         todo                    scan for TODO/FIXME\n\
         serve                   start the HTTP API\n\
         backup/restore          backup and restore state\n\
         git                     git helpers\n\
         pr                      create a pull request\n\
         sandbox                 run commands in a sandbox\n\
         search                  search the codebase\n\
         context                 show session context usage\n\
         model use               switch the active model\n\
         completions             generate shell completions\n\
         upgrade                 self-update and rebuild\n\
         whoami                  show identity summary\n\n\
         CONFIG\n    ~/.config/forge/config.json (override with FORGE_CONFIG)\n\
         Sessions: ~/.local/share/forge/sessions\n\
         Memory:   ~/.local/share/forge/memory.json\n\
         Telemetry: ~/.local/share/forge/telemetry.jsonl\n\n\
         ENVIRONMENT\n    FORGE_API_KEY   API key for the model provider\n\
         FORGE_CONFIG   path to the config file\n\n\
         EXIT STATUS\n    0 success; 1 error; 2 usage; 3 config; 4 tool; 5 invalid args; 6 agent"
    );
}

/// Bash completion script for forge.
fn completions_bash() -> String {
    let cmds = command_names();
    let list = cmds.join(" ");
    format!(
        "_forge() {{\n    local cur=\"${{COMP_WORDS[COMP_CWORD]}}\"\n    COMPREPLY=( $(compgen -W \"{list}\" -- \"$cur\") )\n}}\ncomplete -F _forge forge\n"
    )
}

/// Zsh completion script for forge.
fn completions_zsh() -> String {
    let cmds = command_names();
    let list = cmds.join(" ");
    format!(
        "#compdef forge\n_forge() {{\n    local -a commands\n    commands=({list})\n    _describe 'command' commands\n}}\ncompdef _forge forge\n"
    )
}

/// Fish completion script for forge.
fn completions_fish() -> String {
    let cmds = command_names();
    let mut out = String::new();
    for c in cmds {
        out.push_str(&format!(
            "complete -c forge -f -n '__fish_use_subcommand' -a '{c}'\n"
        ));
    }
    out
}

/// The list of top-level forge subcommand names.
fn command_names() -> Vec<String> {
    let mut names: Vec<String> = Cli::command()
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    names.sort();
    names
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Tools => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let registry = wiring.registry;
            wiring.telemetry.record(
                "tools",
                serde_json::json!({ "count": registry.names().len() }),
            )?;
            for name in registry.names() {
                if let Some(tool) = registry.get(&name) {
                    println!("{}: {}", tool.name(), tool.description());
                }
            }
            Ok(())
        }
        Command::Orchestrate { file, max_turns } => {
            let config = Config::load()?;
            let turns = max_turns.or(config.max_turns).unwrap_or(10);
            let provider = HttpProvider::new(&config.provider)?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let raw = std::fs::read_to_string(&file)?;
            let prompts: Vec<String> = serde_json::from_str(&raw).map_err(|e| {
                crate::error::Error::InvalidArgs(format!("parse {}: {e}", file.display()))
            })?;
            let agent =
                Agent::new(Box::new(provider), wiring.registry, turns).with_hooks(wiring.hooks);
            let results = agent.run_parallel(&prompts)?;
            wiring.telemetry.record(
                "orchestrate",
                serde_json::json!({
                    "sub_agents": results.len(),
                }),
            )?;
            for (i, result) in results.iter().enumerate() {
                println!("=== sub-agent {} ===", i + 1);
                println!("{result}");
            }
            Ok(())
        }
        Command::Workflow {
            file,
            max_turns,
            notify,
        } => {
            let config = Config::load()?;
            let turns = max_turns.or(config.max_turns).unwrap_or(10);
            let provider = HttpProvider::new(&config.provider)?;
            let raw = std::fs::read_to_string(&file)?;
            let workflow: crate::workflow::Workflow = serde_json::from_str(&raw).map_err(|e| {
                crate::error::Error::InvalidArgs(format!("parse {}: {e}", file.display()))
            })?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let agent =
                Agent::new(Box::new(provider), wiring.registry, turns).with_hooks(wiring.hooks);
            let runner = crate::workflow::WorkflowRunner::new(agent);
            let outcome = runner.run(&workflow)?;
            wiring.telemetry.record(
                "workflow",
                serde_json::json!({
                    "name": workflow.name,
                    "tasks": outcome.tasks_run,
                }),
            )?;
            for (id, text) in &outcome.results {
                println!("=== {id} ===\n{text}");
            }
            eprintln!(
                "[forge] {} task(s), {} tokens",
                outcome.tasks_run, outcome.tokens_used
            );
            if notify {
                crate::notify::Notifier::new(true).notify(
                    "forge",
                    &format!(
                        "workflow {} finished ({} tasks)",
                        workflow.name, outcome.tasks_run
                    ),
                )?;
            }
            Ok(())
        }
        Command::Resume {
            session,
            prompt,
            max_turns,
        } => {
            let config = Config::load()?;
            let turns = max_turns.or(config.max_turns).unwrap_or(10);
            let provider = HttpProvider::new(&config.provider)?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let dir = crate::session::default_sessions_dir()?;
            let mut session = crate::session::Session::load(&dir, &session)?;
            let agent =
                Agent::new(Box::new(provider), wiring.registry, turns).with_hooks(wiring.hooks);
            let outcome = agent.run_into(&mut session.messages, &prompt)?;
            session.save(&dir)?;
            wiring.telemetry.record(
                "resume",
                serde_json::json!({
                    "turns": outcome.turns,
                }),
            )?;
            println!("{}", outcome.final_text);
            eprintln!(
                "[forge] {} turn(s), {} tool call(s)",
                outcome.turns, outcome.tool_calls
            );
            Ok(())
        }
        Command::Sessions => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let dir = crate::session::default_sessions_dir()?;
            let ids = crate::session::Session::list(&dir)?;
            wiring
                .telemetry
                .record("sessions", serde_json::json!({ "count": ids.len() }))?;
            if ids.is_empty() {
                println!("no saved sessions");
            } else {
                for id in ids {
                    if let Ok(session) = crate::session::Session::load(&dir, &id) {
                        println!(
                            "{id}\t{} messages\tcreated {}",
                            session.message_count(),
                            session.created_at
                        );
                    } else {
                        println!("{id}");
                    }
                }
            }
            Ok(())
        }
        Command::Export { id, path } => {
            let dir = crate::session::default_sessions_dir()?;
            let session = crate::session::Session::load(&dir, &id)?;
            std::fs::write(&path, session.export()?)?;
            println!("exported session {id} to {}", path.display());
            Ok(())
        }
        Command::Import { path } => {
            let raw = std::fs::read_to_string(&path)?;
            let session = crate::session::Session::import(&raw)?;
            let dir = crate::session::default_sessions_dir()?;
            session.save(&dir)?;
            println!(
                "imported session {} ({} messages)",
                session.id,
                session.message_count()
            );
            Ok(())
        }
        Command::Chat { resume } => {
            let config = Config::load()?;
            crate::tui::run_chat(&config, resume.as_deref())
        }
        Command::Memory { action, key, value } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring
                .telemetry
                .record("memory", serde_json::json!({ "action": action }))?;
            let path = crate::memory::default_memory_path()?;
            let mut memory = crate::memory::Memory::load(path.clone())?;
            match action.as_str() {
                "remember" => {
                    let key =
                        key.ok_or_else(|| crate::error::Error::InvalidArgs("key required".into()))?;
                    let value = value
                        .ok_or_else(|| crate::error::Error::InvalidArgs("value required".into()))?;
                    memory.remember(key, value);
                    memory.save()?;
                    println!("remembered");
                }
                "recall" => {
                    let key =
                        key.ok_or_else(|| crate::error::Error::InvalidArgs("key required".into()))?;
                    match memory.recall(&key) {
                        Some(v) => println!("{v}"),
                        None => println!("(not found)"),
                    }
                }
                "list" => {
                    for (k, v) in memory.all() {
                        println!("{k}: {v}");
                    }
                }
                "clear" => {
                    memory.clear();
                    memory.save()?;
                    println!("memory cleared");
                }
                "export" => {
                    let path = key
                        .ok_or_else(|| crate::error::Error::InvalidArgs("path required".into()))?;
                    let json = memory.export()?;
                    std::fs::write(&path, json)?;
                    println!("exported memory to {path}");
                }
                "import" => {
                    let path = key
                        .ok_or_else(|| crate::error::Error::InvalidArgs("path required".into()))?;
                    let raw = std::fs::read_to_string(&path)?;
                    let count = memory.import(&raw)?;
                    memory.save()?;
                    println!("imported {count} fact(s) from {path}");
                }
                "search" => {
                    let query = key
                        .ok_or_else(|| crate::error::Error::InvalidArgs("query required".into()))?;
                    let hits = memory.search(&query);
                    if hits.is_empty() {
                        println!("(no matches)");
                    } else {
                        for (k, v) in hits {
                            println!("{k}: {v}");
                        }
                    }
                }
                "stats" => {
                    let facts = memory.all();
                    let total_chars: usize = facts.iter().map(|(k, v)| k.len() + v.len()).sum();
                    println!("facts: {}", facts.len());
                    println!("total chars: {total_chars}");
                    println!("file: {}", path.display());
                }
                "remove" => {
                    let key =
                        key.ok_or_else(|| crate::error::Error::InvalidArgs("key required".into()))?;
                    if memory.recall(&key).is_some() {
                        memory.forget(&key);
                        memory.save()?;
                        println!("removed {key}");
                    } else {
                        println!("no fact named {key}");
                    }
                }
                other => {
                    return Err(crate::error::Error::InvalidArgs(format!(
                        "unknown action {other}"
                    )))
                }
            }
            Ok(())
        }
        Command::Cron { file, forever } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let raw = std::fs::read_to_string(&file)?;
            let jobs: Vec<crate::cron::Job> = serde_json::from_str(&raw).map_err(|e| {
                crate::error::Error::InvalidArgs(format!("parse {}: {e}", file.display()))
            })?;
            let mut scheduler = crate::cron::Scheduler::new();
            for job in jobs {
                scheduler.add(job);
            }
            wiring.telemetry.record(
                "cron",
                serde_json::json!({
                    "jobs": scheduler.jobs().len(),
                    "forever": forever,
                }),
            )?;
            if forever {
                scheduler.run_forever()?;
            } else {
                for (name, output) in scheduler.run_once()? {
                    println!("=== {name} ===\n{output}");
                }
            }
            Ok(())
        }
        Command::Review => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let workspace = config.workspace_root();
            let diff = std::process::Command::new("git")
                .args(["diff"])
                .current_dir(&workspace)
                .output()
                .map_err(|e| crate::error::Error::Agent(format!("git diff: {e}")))?;
            let diff_text = String::from_utf8_lossy(&diff.stdout).into_owned();
            let review = crate::review::review_diff(&diff_text);
            wiring.telemetry.record(
                "review",
                serde_json::json!({
                    "findings": review.findings.len(),
                }),
            )?;
            if review.is_clean() {
                println!("no issues found");
            } else {
                for finding in &review.findings {
                    let tag = match finding.severity {
                        crate::review::Severity::Error => "ERROR",
                        crate::review::Severity::Warning => "WARN",
                        crate::review::Severity::Info => "INFO",
                    };
                    println!("[{tag}] {}", finding.message);
                }
            }
            Ok(())
        }
        Command::Mcp { action } => match action {
            McpAction::Call { server, tool, args } => {
                let config = Config::load()?;
                let wiring = crate::wiring::build_wiring(&config)?;
                let mut client = crate::mcp::McpClient::connect(&server, &[])?;
                let tools = client.list_tools()?;
                eprintln!("[forge] MCP tools: {}", tools.join(", "));
                let args: serde_json::Value = serde_json::from_str(&args)
                    .map_err(|e| crate::error::Error::InvalidArgs(format!("bad args: {e}")))?;
                let output = client.call_tool(&tool, args)?;
                wiring
                    .telemetry
                    .record("mcp", serde_json::json!({ "tool": tool }))?;
                println!("{output}");
                Ok(())
            }
            McpAction::Add {
                name,
                command,
                args,
            } => {
                let path = crate::config::config_path()?;
                let mut config = Config::load()?;
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                if config.mcp_servers.iter().any(|s| s.name == name) {
                    return Err(crate::error::Error::InvalidArgs(format!(
                        "MCP server {name} already registered"
                    )));
                }
                config.mcp_servers.push(crate::config::McpServerConfig {
                    name: name.clone(),
                    command,
                    args,
                });
                std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                println!("registered MCP server {name}");
                Ok(())
            }
            McpAction::List => {
                let config = Config::load()?;
                if config.mcp_servers.is_empty() {
                    println!("no MCP servers registered");
                } else {
                    for s in &config.mcp_servers {
                        let mut cmd = s.command.clone();
                        for a in &s.args {
                            cmd.push(' ');
                            cmd.push_str(a);
                        }
                        println!("{}: {cmd}", s.name);
                    }
                }
                Ok(())
            }
            McpAction::Remove { name } => {
                let path = crate::config::config_path()?;
                let mut config = Config::load()?;
                let before = config.mcp_servers.len();
                config.mcp_servers.retain(|s| s.name != name);
                if config.mcp_servers.len() == before {
                    println!("no MCP server named {name}");
                } else {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                    println!("removed MCP server {name}");
                }
                Ok(())
            }
            McpAction::Test { name } => {
                let config = Config::load()?;
                let server = config
                    .mcp_servers
                    .iter()
                    .find(|s| s.name == name)
                    .ok_or_else(|| {
                        crate::error::Error::InvalidArgs(format!("no MCP server named {name}"))
                    })?;
                let args: Vec<&str> = server.args.iter().map(String::as_str).collect();
                let mut client = crate::mcp::McpClient::connect(&server.command, &args)?;
                let tools = client.list_tools()?;
                println!("MCP server {name} OK — {} tool(s):", tools.len());
                for tool in &tools {
                    println!("  - {tool}");
                }
                Ok(())
            }
        },
        Command::Diff {
            staged,
            stat,
            name_only,
        } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record(
                "diff",
                serde_json::json!({ "staged": staged, "stat": stat, "name_only": name_only }),
            )?;
            let ws = config.workspace_root();
            let base: &[&str] = if staged {
                &["diff", "--staged"]
            } else {
                &["diff", "HEAD"]
            };
            let mut args: Vec<&str> = base.to_vec();
            if stat {
                args.push("--stat");
            }
            if name_only {
                args.push("--name-only");
            }
            let output = std::process::Command::new("git")
                .args(&args)
                .current_dir(&ws)
                .output()?;
            let text = String::from_utf8_lossy(&output.stdout).into_owned();
            if text.trim().is_empty() {
                println!("no changes");
            } else {
                println!("{text}");
            }
            Ok(())
        }
        Command::Commit {
            message,
            all,
            amend,
        } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring
                .telemetry
                .record("commit", serde_json::json!({ "amend": amend }))?;
            let ws = config.workspace_root();
            if all {
                let out = std::process::Command::new("git")
                    .args(["add", "-A"])
                    .current_dir(&ws)
                    .output()?;
                if !out.status.success() {
                    return Err(crate::error::Error::Tool(format!(
                        "git add failed: {}",
                        String::from_utf8_lossy(&out.stderr)
                    )));
                }
            }
            let msg = match message {
                Some(m) => m,
                None => {
                    let diff = std::process::Command::new("git")
                        .args(["diff", "--cached"])
                        .current_dir(&ws)
                        .output()?;
                    let diff_text = String::from_utf8_lossy(&diff.stdout).into_owned();
                    if diff_text.trim().is_empty() {
                        return Err(crate::error::Error::InvalidArgs(
                            "nothing staged to commit; use --all or stage changes first".into(),
                        ));
                    }
                    generate_commit_message(&config, &diff_text)?
                }
            };
            let mut args = vec!["commit", "-m", &msg];
            if amend {
                args.push("--amend");
            }
            let out = std::process::Command::new("git")
                .args(&args)
                .current_dir(&ws)
                .output()?;
            let text = String::from_utf8_lossy(&out.stdout).into_owned();
            if !out.status.success() {
                return Err(crate::error::Error::Tool(format!(
                    "git commit failed: {}{}",
                    text,
                    String::from_utf8_lossy(&out.stderr)
                )));
            }
            println!("{text}");
            Ok(())
        }
        Command::Build => {
            let config = Config::load()?;
            run_project_command(&config, "build")
        }
        Command::Test => {
            let config = Config::load()?;
            run_project_command(&config, "test")
        }
        Command::Lint => {
            let config = Config::load()?;
            run_project_command(&config, "lint")
        }
        Command::Format => {
            let config = Config::load()?;
            run_project_command(&config, "format")
        }
        Command::Check => {
            let config = Config::load()?;
            run_project_command(&config, "check")
        }
        Command::Agent { prompt, max_turns } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("agent", serde_json::json!({}))?;
            let turns = max_turns.or(config.max_turns).unwrap_or(10);
            let provider = HttpProvider::new(&config.provider)?;
            let agent =
                Agent::new(Box::new(provider), wiring.registry, turns).with_hooks(wiring.hooks);
            let dir = crate::session::default_sessions_dir()?;
            let id = format!(
                "agent-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            );
            let mut session = crate::session::Session::new(&id);
            let outcome = agent.run_into(&mut session.messages, &prompt)?;
            session.save(&dir)?;
            println!("{}", outcome.final_text);
            eprintln!(
                "[forge] {} turn(s), {} tool call(s), session {}",
                outcome.turns, outcome.tool_calls, session.id
            );
            Ok(())
        }
        Command::Audit => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("audit", serde_json::json!({}))?;
            let ws = config.workspace_root();
            let (bin, args): (&str, &[&str]) = if ws.join("Cargo.toml").exists() {
                ("cargo", &["audit"])
            } else if ws.join("package.json").exists() {
                ("npm", &["audit"])
            } else {
                return Err(crate::error::Error::InvalidArgs(
                    "no Cargo.toml or package.json found to audit".into(),
                ));
            };
            eprintln!("[forge] audit: {bin} {}", args.join(" "));
            let status = std::process::Command::new(bin)
                .args(args)
                .current_dir(&ws)
                .status()?;
            if status.success() {
                Ok(())
            } else {
                Err(crate::error::Error::Tool(format!(
                    "audit found issues (exit {status})"
                )))
            }
        }
        Command::Todo => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("todo", serde_json::json!({}))?;
            let ws = config.workspace_root();
            scan_todos(&ws)?;
            Ok(())
        }
        Command::Browser {
            url,
            eval,
            click,
            r#type,
            screenshot,
            wait,
            text,
        } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let browser = crate::browser::Browser::launch()?;
            let target = browser.open(&url)?;
            wiring
                .telemetry
                .record("browser", serde_json::json!({ "url": url }))?;
            println!("opened {url} (target {})", target.id);
            if let Some(ms) = wait {
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
            if let Some(js) = eval {
                let result = browser.evaluate(&target, &js)?;
                println!("eval result: {result}");
            }
            if let Some(coord) = click {
                let mut parts = coord.split(',');
                let x: i32 = parts
                    .next()
                    .and_then(|s| s.trim().parse().ok())
                    .ok_or_else(|| crate::error::Error::InvalidArgs("--click needs x,y".into()))?;
                let y: i32 = parts
                    .next()
                    .and_then(|s| s.trim().parse().ok())
                    .ok_or_else(|| crate::error::Error::InvalidArgs("--click needs x,y".into()))?;
                browser.click(&target, x, y)?;
                println!("clicked {x},{y}");
            }
            if let Some(text) = r#type {
                browser.type_text(&target, &text)?;
                println!("typed");
            }
            if let Some(path) = screenshot {
                let data = browser.screenshot(&target)?;
                use base64::Engine as _;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .map_err(|e| crate::error::Error::Agent(format!("decode screenshot: {e}")))?;
                std::fs::write(&path, bytes)?;
                println!("screenshot saved to {path}");
            }
            if text {
                let body_text = browser.get_text(&target)?;
                println!("page text:\n{body_text}");
            }
            for tab in browser.list()? {
                println!("tab: {tab}");
            }
            Ok(())
        }
        Command::Desktop { action, args } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let desktop = crate::desktop::Desktop::new();
            wiring
                .telemetry
                .record("desktop", serde_json::json!({ "action": action }))?;
            match action.as_str() {
                "screenshot" => {
                    let path = args.first().ok_or_else(|| {
                        crate::error::Error::InvalidArgs("screenshot needs a path".into())
                    })?;
                    desktop.screenshot(std::path::Path::new(path))?;
                    println!("screenshot saved to {path}");
                }
                "click" => {
                    let x: i32 = args.first().and_then(|s| s.parse().ok()).ok_or_else(|| {
                        crate::error::Error::InvalidArgs("click needs x y".into())
                    })?;
                    let y: i32 = args.get(1).and_then(|s| s.parse().ok()).ok_or_else(|| {
                        crate::error::Error::InvalidArgs("click needs x y".into())
                    })?;
                    desktop.click(x, y)?;
                    println!("clicked {x},{y}");
                }
                "type" => {
                    let text = args.join(" ");
                    desktop.type_text(&text)?;
                    println!("typed");
                }
                other => {
                    return Err(crate::error::Error::InvalidArgs(format!(
                        "unknown desktop action {other}"
                    )))
                }
            }
            Ok(())
        }
        Command::Setup => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("setup", serde_json::json!({}))?;
            // Config.
            let config_path = crate::config::config_path()?;
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let config = r#"{
  "workspace": ".",
  "provider": {
    "base_url": "https://api.openai.com/v1",
    "model": "gpt-4o-mini",
    "api_key": ""
  },
  "max_turns": 10,
  "mcp_servers": [],
  "plugins_dir": ".forge/plugins",
  "hooks": [],
  "telemetry": true
}
"#;
            std::fs::write(&config_path, config)?;
            println!("wrote config: {}", config_path.display());

            // Sample workflow.
            let workspace = Config::load()?.workspace_root();
            let forge_dir = workspace.join(".forge");
            std::fs::create_dir_all(&forge_dir)?;
            let workflow = r#"{
  "name": "research",
  "max_workers": 3,
  "max_tokens": 100000,
  "tasks": [
    { "id": "discover", "prompt": "List the modules in this project.", "depends_on": [], "phase": "discover" },
    { "id": "review", "prompt": "Review the core module for issues.", "depends_on": ["discover"], "phase": "review" },
    { "id": "synthesize", "prompt": "Summarize the review findings.", "depends_on": ["review"], "phase": "synthesize" }
  ]
}
"#;
            std::fs::write(forge_dir.join("workflow.json"), workflow)?;
            println!(
                "wrote sample workflow: {}",
                forge_dir.join("workflow.json").display()
            );

            // Sample cron.
            let cron = r#"[
  { "name": "daily-status", "interval_secs": 86400, "command": "git status --short" }
]
"#;
            std::fs::write(forge_dir.join("cron.json"), cron)?;
            println!(
                "wrote sample cron: {}",
                forge_dir.join("cron.json").display()
            );

            println!("\nNext: set provider.api_key (or FORGE_API_KEY), then run:\n  forge run \"hello\"\n  forge workflow .forge/workflow.json\n  forge cron .forge/cron.json");
            Ok(())
        }
        Command::Doctor { fix, json } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring
                .telemetry
                .record("doctor", serde_json::json!({ "fix": fix, "json": json }))?;
            let mut ok = true;
            let mut checks: Vec<serde_json::Value> = Vec::new();
            let mut check = |name: &str, pass: bool, detail: &str| {
                if json {
                    checks.push(serde_json::json!({
                        "name": name,
                        "ok": pass,
                        "detail": detail,
                    }));
                } else {
                    println!("[{}] {name}: {detail}", if pass { "ok" } else { "MISSING" });
                }
                if !pass {
                    ok = false;
                }
            };
            check(
                "config",
                config.provider.model.is_some(),
                "provider model configured",
            );
            check(
                "api key",
                std::env::var("FORGE_API_KEY").is_ok() || config.provider.api_key.is_some(),
                "FORGE_API_KEY or provider.api_key",
            );
            check("git", which("git"), "git binary");
            check(
                "browser",
                which("google-chrome") || which("chromium"),
                "chrome/chromium",
            );
            if cfg!(target_os = "macos") {
                check("desktop", which("cliclick"), "cliclick (desktop control)");
            }
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": ok,
                        "checks": checks,
                    }))?
                );
            } else if ok {
                println!("\nall checks passed");
            } else {
                println!("\nsome checks failed — see above");
            }
            if fix {
                // Attempt to fix config issues: write a config if none exists.
                let path = crate::config::config_path()?;
                if !path.exists() {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    let sample = r#"{
  "workspace": ".",
  "provider": {
    "base_url": "https://api.openai.com/v1",
    "model": "gpt-4o-mini",
    "api_key": ""
  },
  "max_turns": 10,
  "mcp_servers": [],
  "plugins_dir": ".forge/plugins",
  "hooks": [],
  "telemetry": true
}
"#;
                    std::fs::write(&path, sample)?;
                    println!("wrote a default config to {}", path.display());
                } else {
                    println!("config already exists at {}", path.display());
                }
            }
            Ok(())
        }
        Command::Info => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let registry = wiring.registry;
            wiring.telemetry.record("info", serde_json::json!({}))?;
            let sessions = crate::session::Session::list(&crate::session::default_sessions_dir()?)?;
            println!("forge {}", env!("CARGO_PKG_VERSION"));
            println!("workspace: {}", config.workspace_root().display());
            println!(
                "model: {}",
                config.provider.model.as_deref().unwrap_or("(none)")
            );
            println!("tools: {}", registry.names().len());
            println!("sessions: {}", sessions.len());
            Ok(())
        }
        Command::Config { action } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let action_name = match &action {
                None => "show",
                Some(ConfigAction::Show) => "show",
                Some(ConfigAction::Reset) => "reset",
                Some(ConfigAction::Get { .. }) => "get",
                Some(ConfigAction::Set { .. }) => "set",
                Some(ConfigAction::Validate) => "validate",
                Some(ConfigAction::Path) => "path",
                Some(ConfigAction::Unset { .. }) => "unset",
                Some(ConfigAction::Commands) => "commands",
                Some(ConfigAction::List) => "list",
            };
            wiring
                .telemetry
                .record("config", serde_json::json!({ "action": action_name }))?;
            match action {
                None | Some(ConfigAction::Show) => {
                    let mut redacted = config.clone();
                    if let Some(key) = redacted.provider.api_key.as_mut() {
                        if !key.is_empty() {
                            *key = "***".to_string();
                        }
                    }
                    println!("{}", serde_json::to_string_pretty(&redacted)?);
                }
                Some(ConfigAction::Reset) => {
                    let path = crate::config::config_path()?;
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    let sample = r#"{
  "workspace": ".",
  "provider": {
    "base_url": "https://api.openai.com/v1",
    "model": "gpt-4o-mini",
    "api_key": ""
  },
  "max_turns": 10,
  "mcp_servers": [],
  "plugins_dir": ".forge/plugins",
  "hooks": [],
  "telemetry": true
}
"#;
                    std::fs::write(&path, sample)?;
                    println!("reset config to defaults at {}", path.display());
                }
                Some(ConfigAction::Get { key }) => {
                    let value = match key.as_str() {
                        "provider.model" => config
                            .provider
                            .model
                            .map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null),
                        "provider.base_url" => config
                            .provider
                            .base_url
                            .map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null),
                        "provider.api_key" => {
                            if config.provider.api_key.is_some() {
                                serde_json::Value::String("***".into())
                            } else {
                                serde_json::Value::Null
                            }
                        }
                        "max_turns" => config
                            .max_turns
                            .map(|t| serde_json::Value::Number(t.into()))
                            .unwrap_or(serde_json::Value::Null),
                        "workspace" => config
                            .workspace
                            .map(|p| serde_json::Value::String(p.display().to_string()))
                            .unwrap_or(serde_json::Value::Null),
                        "plugins_dir" => config
                            .plugins_dir
                            .map(|p| serde_json::Value::String(p.display().to_string()))
                            .unwrap_or(serde_json::Value::Null),
                        "commands.build" => config
                            .commands
                            .build
                            .clone()
                            .map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null),
                        "commands.test" => config
                            .commands
                            .test
                            .clone()
                            .map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null),
                        "commands.lint" => config
                            .commands
                            .lint
                            .clone()
                            .map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null),
                        "commands.format" => config
                            .commands
                            .format
                            .clone()
                            .map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null),
                        "commands.check" => config
                            .commands
                            .check
                            .clone()
                            .map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null),
                        "telemetry" => serde_json::Value::Bool(config.telemetry),
                        other => {
                            return Err(crate::error::Error::InvalidArgs(format!(
                                "unknown config key {other}"
                            )))
                        }
                    };
                    println!("{value}");
                }
                Some(ConfigAction::Set { key, value }) => {
                    let path = crate::config::config_path()?;
                    let mut config = config;
                    match key.as_str() {
                        "provider.model" => config.provider.model = Some(value.clone()),
                        "provider.base_url" => config.provider.base_url = Some(value.clone()),
                        "provider.api_key" => config.provider.api_key = Some(value.clone()),
                        "max_turns" => {
                            config.max_turns = Some(value.parse().map_err(|_| {
                                crate::error::Error::InvalidArgs(
                                    "max_turns must be a number".into(),
                                )
                            })?)
                        }
                        "workspace" => config.workspace = Some(std::path::PathBuf::from(&value)),
                        "plugins_dir" => {
                            config.plugins_dir = Some(std::path::PathBuf::from(&value))
                        }
                        "commands.build" => config.commands.build = Some(value.clone()),
                        "commands.test" => config.commands.test = Some(value.clone()),
                        "commands.lint" => config.commands.lint = Some(value.clone()),
                        "commands.format" => config.commands.format = Some(value.clone()),
                        "commands.check" => config.commands.check = Some(value.clone()),
                        "telemetry" => config.telemetry = value == "true" || value == "on",
                        other => {
                            return Err(crate::error::Error::InvalidArgs(format!(
                                "unknown config key {other}"
                            )))
                        }
                    }
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                    println!("set {key} = {value}");
                }
                Some(ConfigAction::Validate) => {
                    config.validate()?;
                    println!("config valid");
                }
                Some(ConfigAction::Path) => {
                    println!("{}", crate::config::config_path()?.display());
                }
                Some(ConfigAction::Unset { key }) => {
                    let path = crate::config::config_path()?;
                    let mut config = config;
                    match key.as_str() {
                        "provider.model" => config.provider.model = None,
                        "provider.base_url" => config.provider.base_url = None,
                        "provider.api_key" => config.provider.api_key = None,
                        "max_turns" => config.max_turns = None,
                        "workspace" => config.workspace = None,
                        "plugins_dir" => config.plugins_dir = None,
                        "commands.build" => config.commands.build = None,
                        "commands.test" => config.commands.test = None,
                        "commands.lint" => config.commands.lint = None,
                        "commands.format" => config.commands.format = None,
                        "commands.check" => config.commands.check = None,
                        "telemetry" => config.telemetry = true,
                        other => {
                            return Err(crate::error::Error::InvalidArgs(format!(
                                "unknown config key {other}"
                            )))
                        }
                    }
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                    println!("unset {key}");
                }
                Some(ConfigAction::Commands) => {
                    let ws = config.workspace_root();
                    for step in ["build", "test", "lint", "format", "check"] {
                        println!("{step}: {}", config.commands.resolve(step, &ws));
                    }
                }
                Some(ConfigAction::List) => {
                    println!("workspace: {}", config.workspace_root().display());
                    println!(
                        "model: {}",
                        config.provider.model.as_deref().unwrap_or("(none)")
                    );
                    println!(
                        "base_url: {}",
                        config.provider.base_url.as_deref().unwrap_or("(default)")
                    );
                    println!(
                        "api_key: {}",
                        if config.provider.api_key.is_some() {
                            "set"
                        } else {
                            "unset"
                        }
                    );
                    println!("max_turns: {:?}", config.max_turns);
                    println!(
                        "plugins_dir: {}",
                        config
                            .plugins_dir
                            .as_deref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "(default)".into())
                    );
                    println!("telemetry: {}", config.telemetry);
                    println!("aliases: {}", config.aliases.len());
                    println!("mcp_servers: {}", config.mcp_servers.len());
                    println!("saved_providers: {}", config.saved_providers.len());
                }
            }
            Ok(())
        }
        Command::Session { action } => {
            let dir = crate::session::default_sessions_dir()?;
            match action {
                SessionAction::List => {
                    let ids = crate::session::Session::list(&dir)?;
                    if ids.is_empty() {
                        println!("no sessions");
                    } else {
                        for sid in ids {
                            if let Ok(session) = crate::session::Session::load(&dir, &sid) {
                                println!(
                                    "{sid}  {} message(s), ~{} token(s)",
                                    session.message_count(),
                                    session.token_usage()
                                );
                            } else {
                                println!("{sid}");
                            }
                        }
                    }
                }
                SessionAction::Show { id } => {
                    let session = crate::session::Session::load(&dir, &id)?;
                    println!("id: {}", session.id);
                    println!("created: {}", session.created_at);
                    println!("messages: {}", session.message_count());
                    println!("tokens: ~{}", session.token_usage());
                }
                SessionAction::Delete { id } => {
                    let path = dir.join(format!("{id}.json"));
                    if path.exists() {
                        std::fs::remove_file(&path)?;
                        println!("deleted session {id}");
                    } else {
                        println!("no session {id}");
                    }
                }
                SessionAction::Rename { id, new } => {
                    let old_path = dir.join(format!("{id}.json"));
                    let new_path = dir.join(format!("{new}.json"));
                    if old_path.exists() {
                        std::fs::rename(&old_path, &new_path)?;
                        println!("renamed session {id} to {new}");
                    } else {
                        println!("no session {id}");
                    }
                }
                SessionAction::Export { id, path } => {
                    let session = crate::session::Session::load(&dir, &id)?;
                    let raw = session.export()?;
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&path, raw)?;
                    println!("exported session {id} to {}", path.display());
                }
                SessionAction::Import { path } => {
                    let raw = std::fs::read_to_string(&path)?;
                    let session = crate::session::Session::import(&raw)?;
                    session.save(&dir)?;
                    println!(
                        "imported session {} ({} messages)",
                        session.id,
                        session.message_count()
                    );
                }
            }
            Ok(())
        }
        Command::Update => {
            // Compare the local version against the remote main branch.
            let local = env!("CARGO_PKG_VERSION");
            let remote = std::process::Command::new("git")
                .args([
                    "ls-remote",
                    "https://github.com/gnanam1990/forge.git",
                    "main",
                ])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_default();
            if remote.is_empty() {
                println!("forge {local} (could not reach remote)");
            } else {
                println!("forge {local} — remote main: {remote}");
                println!("run `git pull` in the forge repo to update.");
            }
            Ok(())
        }
        Command::Log => {
            let path = crate::telemetry::default_telemetry_path()?;
            if let Ok(raw) = std::fs::read_to_string(&path) {
                if raw.trim().is_empty() {
                    println!("no log entries");
                } else {
                    println!("{raw}");
                }
            } else {
                println!("no log file at {}", path.display());
            }
            Ok(())
        }
        Command::Telemetry { action } => match action {
            TelemetryAction::On => {
                let path = crate::config::config_path()?;
                let mut config = Config::load()?;
                config.telemetry = true;
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                println!("telemetry on");
                Ok(())
            }
            TelemetryAction::Off => {
                let path = crate::config::config_path()?;
                let mut config = Config::load()?;
                config.telemetry = false;
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                println!("telemetry off");
                Ok(())
            }
            TelemetryAction::Export { path } => {
                let config = Config::load()?;
                let wiring = crate::wiring::build_wiring(&config)?;
                wiring
                    .telemetry
                    .record("telemetry_export", serde_json::json!({}))?;
                let src = crate::telemetry::default_telemetry_path()?;
                let raw = std::fs::read_to_string(&src).unwrap_or_default();
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, raw)?;
                println!("exported telemetry to {}", path.display());
                Ok(())
            }
            TelemetryAction::Clear => {
                let path = crate::telemetry::default_telemetry_path()?;
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, "")?;
                println!("telemetry cleared");
                Ok(())
            }
            TelemetryAction::Stats => {
                let path = crate::telemetry::default_telemetry_path()?;
                let raw = std::fs::read_to_string(&path).unwrap_or_default();
                let mut counts: std::collections::BTreeMap<String, usize> =
                    std::collections::BTreeMap::new();
                let mut total = 0usize;
                for line in raw.lines() {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(event) = value.get("event").and_then(|e| e.as_str()) {
                            *counts.entry(event.to_string()).or_insert(0) += 1;
                            total += 1;
                        }
                    }
                }
                if total == 0 {
                    println!("no telemetry events recorded");
                } else {
                    println!("total events: {total}");
                    for (event, count) in &counts {
                        println!("{event}: {count}");
                    }
                }
                Ok(())
            }
            TelemetryAction::Path => {
                println!("{}", crate::telemetry::default_telemetry_path()?.display());
                Ok(())
            }
        },
        Command::Alias { name, command } => {
            let path = crate::config::config_path()?;
            let mut config = Config::load()?;
            // `alias remove <name>` is a remove; `alias <name> <command>` is an add.
            if name.as_deref() == Some("remove") {
                let target = command.ok_or_else(|| {
                    crate::error::Error::InvalidArgs("alias name required".into())
                })?;
                if config.aliases.remove(&target).is_some() {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                    println!("removed alias {target}");
                } else {
                    println!("no alias named {target}");
                }
                return Ok(());
            }
            match (name, command) {
                (None, _) => {
                    // List all aliases.
                    if config.aliases.is_empty() {
                        println!("no aliases defined");
                    } else {
                        let mut names: Vec<&String> = config.aliases.keys().collect();
                        names.sort();
                        for n in names {
                            println!("{n} = {}", config.aliases[n]);
                        }
                    }
                }
                (Some(name), Some(command)) => {
                    config.aliases.insert(name.clone(), command.clone());
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                    println!("alias {name} = {command}");
                }
                (Some(name), None) => {
                    if config.aliases.remove(&name).is_some() {
                        if let Some(parent) = path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                        println!("removed alias {name}");
                    } else {
                        println!("no alias named {name}");
                    }
                }
            }
            Ok(())
        }
        Command::Plugins => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let mut registry = Registry::builtin();
            let dir = config
                .plugins_dir
                .clone()
                .unwrap_or_else(|| config.workspace_root().join(".forge").join("plugins"));
            let count = crate::plugin::load_plugins_from_dir(&dir, &mut registry)?;
            wiring
                .telemetry
                .record("plugins", serde_json::json!({ "count": count }))?;
            if count == 0 {
                println!("no plugins loaded from {}", dir.display());
            } else {
                println!("loaded {count} plugin tool(s) from {}", dir.display());
            }
            Ok(())
        }
        Command::Plugin { action, arg } => {
            let config = Config::load()?;
            let dir = config
                .plugins_dir
                .clone()
                .unwrap_or_else(|| config.workspace_root().join(".forge").join("plugins"));
            let state_path = dir.join("state.json");
            let mut registry = crate::plugin::PluginRegistry::new(state_path);
            registry.load_dir(&dir)?;
            match action.as_str() {
                "list" => {
                    for entry in registry.list() {
                        println!(
                            "{} [{}] {} tool(s)",
                            entry.name,
                            if entry.enabled { "enabled" } else { "disabled" },
                            entry.tools.len()
                        );
                    }
                }
                "enable" => {
                    let name = arg.ok_or_else(|| {
                        crate::error::Error::InvalidArgs("plugin name required".into())
                    })?;
                    registry.enable(&name)?;
                    println!("enabled {name}");
                }
                "disable" => {
                    let name = arg.ok_or_else(|| {
                        crate::error::Error::InvalidArgs("plugin name required".into())
                    })?;
                    registry.disable(&name)?;
                    println!("disabled {name}");
                }
                "add" => {
                    let file = arg.ok_or_else(|| {
                        crate::error::Error::InvalidArgs("plugin file required".into())
                    })?;
                    std::fs::create_dir_all(&dir)?;
                    let dest =
                        dir.join(std::path::Path::new(&file).file_name().unwrap_or_default());
                    std::fs::copy(&file, &dest)?;
                    registry.load_dir(&dir)?;
                    println!("added plugin from {file}");
                }
                "docs" => {
                    println!(
                        "forge plugins\n\n\
                         A plugin is a JSON file in the plugins directory:\n\
                         {{\n  \"name\": \"my-plugin\",\n  \"tools\": [\n    {{\n      \"name\": \"my_tool\",\n      \
                         \"command\": \"my-command\",\n      \"description\": \"does something\"\n    }}\n  ]\n}}\n\n\
                         Commands:\n  forge plugin list              list plugins\n  \
                         forge plugin enable <name>    enable a plugin\n  \
                         forge plugin disable <name>   disable a plugin\n  \
                         forge plugin add <file>      add a plugin file"
                    );
                }
                "info" => {
                    let name = arg.ok_or_else(|| {
                        crate::error::Error::InvalidArgs("plugin name required".into())
                    })?;
                    let entry =
                        registry
                            .list()
                            .iter()
                            .find(|p| p.name == name)
                            .ok_or_else(|| {
                                crate::error::Error::Config(format!("unknown plugin {name}"))
                            })?;
                    println!("name: {}", entry.name);
                    println!(
                        "status: {}",
                        if entry.enabled { "enabled" } else { "disabled" }
                    );
                    println!("tools ({}):", entry.tools.len());
                    for tool in &entry.tools {
                        println!("  - {tool}");
                    }
                }
                other => {
                    return Err(crate::error::Error::InvalidArgs(format!(
                        "unknown plugin action {other}"
                    )))
                }
            }
            Ok(())
        }
        Command::Effort { level } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let effort = crate::posture::Effort::parse(&level).ok_or_else(|| {
                crate::error::Error::InvalidArgs(format!(
                    "unknown effort {level}; expected auto, balanced, thorough, or zeromaxing"
                ))
            })?;
            let posture = crate::posture::Posture::from_effort(effort);
            let auto = crate::posture::Posture::from_effort(crate::posture::Effort::Auto);
            wiring
                .telemetry
                .record("effort", serde_json::json!({ "effort": level }))?;
            println!("effort: {}", posture.effort.as_str());
            println!("delta: {}", posture.delta(&auto));
            Ok(())
        }
        Command::Plan { file } => {
            let config = Config::load()?;
            let turns = config.max_turns.unwrap_or(10);
            let provider = HttpProvider::new(&config.provider)?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let raw = std::fs::read_to_string(&file)?;
            let plan: crate::plan::Plan = serde_json::from_str(&raw).map_err(|e| {
                crate::error::Error::InvalidArgs(format!("parse {}: {e}", file.display()))
            })?;
            let agent =
                Agent::new(Box::new(provider), wiring.registry, turns).with_hooks(wiring.hooks);
            let runner =
                crate::plan_exec::PlanRunner::new(agent).with_progress(Box::new(|id, status| {
                    eprintln!("[plan] {id}: {status}");
                }));
            let outcome = runner.run(&plan)?;
            wiring.telemetry.record(
                "plan",
                serde_json::json!({
                    "name": plan.name,
                    "status": format!("{:?}", outcome.status),
                    "tasks": outcome.tasks_run,
                }),
            )?;
            crate::notify::Notifier::new(true).notify(
                "forge",
                &format!("plan {} finished: {:?}", plan.name, outcome.status),
            )?;
            println!("status: {:?}", outcome.status);
            for (id, text) in &outcome.results {
                println!("=== {id} ===\n{text}");
            }
            eprintln!(
                "[forge] {} task(s), {} tokens, status {:?}",
                outcome.tasks_run, outcome.tokens_used, outcome.status
            );
            Ok(())
        }
        Command::Version => {
            println!("forge {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Models => {
            for model in crate::models::MODELS {
                println!("{model}");
            }
            Ok(())
        }
        Command::Provider { action } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let action_name = match &action {
                ProviderAction::Show => "show",
                ProviderAction::Set { .. } => "set",
                ProviderAction::List => "list",
                ProviderAction::Add { .. } => "add",
                ProviderAction::Remove { .. } => "remove",
            };
            wiring
                .telemetry
                .record("provider", serde_json::json!({ "action": action_name }))?;
            match action {
                ProviderAction::Show => {
                    println!(
                        "base_url: {}",
                        config.provider.base_url.as_deref().unwrap_or("(default)")
                    );
                    println!(
                        "model: {}",
                        config.provider.model.as_deref().unwrap_or("(none)")
                    );
                    println!(
                        "api_key: {}",
                        if config.provider.api_key.is_some() {
                            "set"
                        } else {
                            "unset"
                        }
                    );
                }
                ProviderAction::Set { model } => {
                    let path = crate::config::config_path()?;
                    let mut config = config;
                    config.provider.model = Some(model.clone());
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                    println!("set model to {model}");
                }
                ProviderAction::List => {
                    println!(
                        "active: {}",
                        config.provider.model.as_deref().unwrap_or("(none)")
                    );
                    for (i, provider) in config.saved_providers.iter().enumerate() {
                        println!(
                            "  {i}: {} ({})",
                            provider.model.as_deref().unwrap_or("(unnamed)"),
                            provider.base_url.as_deref().unwrap_or("(default)")
                        );
                    }
                }
                ProviderAction::Add {
                    name,
                    model,
                    base_url,
                } => {
                    let path = crate::config::config_path()?;
                    let mut config = config;
                    config.saved_providers.push(crate::config::ProviderConfig {
                        model: Some(model.clone()),
                        base_url: base_url.clone(),
                        ..Default::default()
                    });
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                    println!(
                        "added provider {}{}",
                        name.as_deref().unwrap_or(&model),
                        base_url
                            .as_deref()
                            .map(|u| format!(" ({u})"))
                            .unwrap_or_default()
                    );
                }
                ProviderAction::Remove { target } => {
                    let path = crate::config::config_path()?;
                    let mut config = config;
                    let before = config.saved_providers.len();
                    if let Ok(idx) = target.parse::<usize>() {
                        if idx < config.saved_providers.len() {
                            config.saved_providers.remove(idx);
                        }
                    } else {
                        config
                            .saved_providers
                            .retain(|p| p.model.as_deref() != Some(target.as_str()));
                    }
                    if config.saved_providers.len() == before {
                        println!("no saved provider {target}");
                    } else {
                        if let Some(parent) = path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                        println!("removed provider {target}");
                    }
                }
            }
            Ok(())
        }
        Command::Docs => {
            println!(
                "forge — a coding agent in Rust\n\n\
                 Quick start:\n  forge setup          write config + samples\n  \
                 forge run \"hello\"   run the agent\n  forge chat          interactive TUI\n\n\
                 See `forge help` for all commands, and README.md for the full guide."
            );
            Ok(())
        }
        Command::Stats => {
            let path = crate::telemetry::default_telemetry_path()?;
            let mut counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            if let Ok(raw) = std::fs::read_to_string(&path) {
                for line in raw.lines() {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(event) = value.get("event").and_then(serde_json::Value::as_str)
                        {
                            *counts.entry(event.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
            if counts.is_empty() {
                println!("no usage data yet");
            } else {
                for (event, count) in counts {
                    println!("{event}: {count}");
                }
            }
            Ok(())
        }
        Command::Env { json } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring
                .telemetry
                .record("env", serde_json::json!({ "json": json }))?;
            if json {
                let info = serde_json::json!({
                    "version": env!("CARGO_PKG_VERSION"),
                    "os": std::env::consts::OS,
                    "arch": std::env::consts::ARCH,
                    "cwd": std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_else(|_| "?".into()),
                    "home": std::env::var("HOME").unwrap_or_else(|_| "?".into()),
                    "config": crate::config::config_path().map(|p| p.display().to_string()).unwrap_or_else(|_| "?".into()),
                });
                println!("{}", serde_json::to_string_pretty(&info)?);
                return Ok(());
            }
            println!("forge {}", env!("CARGO_PKG_VERSION"));
            println!("os: {}", std::env::consts::OS);
            println!("arch: {}", std::env::consts::ARCH);
            println!(
                "cwd: {}",
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "?".into())
            );
            println!(
                "home: {}",
                std::env::var("HOME").unwrap_or_else(|_| "?".into())
            );
            println!(
                "config: {}",
                crate::config::config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "?".into())
            );
            Ok(())
        }
        Command::Shell => {
            let mut session = crate::tools::terminal::TerminalSession::spawn()?;
            println!("forge shell — type commands, or /exit to quit.");
            loop {
                print!("$ ");
                std::io::stdout().flush()?;
                let mut line = String::new();
                if std::io::stdin().read_line(&mut line)? == 0 {
                    break;
                }
                let line = line.trim();
                if line == "/exit" || line == "/quit" {
                    break;
                }
                if line.is_empty() {
                    continue;
                }
                session.send(line)?;
                std::thread::sleep(std::time::Duration::from_millis(200));
                let output = session.read()?;
                print!("{output}");
                std::io::stdout().flush()?;
            }
            Ok(())
        }
        Command::Watch { dir, command } => {
            println!(
                "watching {} — run `{}` on change (Ctrl+C to stop)",
                dir.display(),
                command
            );
            let mut last: std::collections::HashMap<std::path::PathBuf, std::time::SystemTime> =
                std::collections::HashMap::new();
            loop {
                let mut changed = false;
                for entry in walkdir::WalkDir::new(&dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(modified) = meta.modified() {
                                if last
                                    .get(entry.path())
                                    .map(|t| *t != modified)
                                    .unwrap_or(true)
                                {
                                    last.insert(entry.path().to_path_buf(), modified);
                                    changed = true;
                                }
                            }
                        }
                    }
                }
                if changed {
                    let output = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&command)
                        .output()?;
                    print!("{}", String::from_utf8_lossy(&output.stdout));
                    std::io::stdout().flush()?;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        Command::Benchmark => {
            // A simple benchmark: run the glob tool over the workspace repeatedly.
            let config = Config::load()?;
            let registry = Registry::builtin();
            let ctx = crate::tools::ToolContext::new(config.workspace_root());
            let glob = registry.get("glob").unwrap();
            let start = std::time::Instant::now();
            let mut iterations = 0usize;
            while start.elapsed() < std::time::Duration::from_secs(2) {
                let _ = glob.run(&serde_json::json!({"pattern": "**/*.rs"}), &ctx);
                iterations += 1;
            }
            let elapsed = start.elapsed().as_secs_f64();
            println!(
                "glob benchmark: {iterations} iterations in {elapsed:.2}s ({:.0}/s)",
                iterations as f64 / elapsed
            );
            Ok(())
        }
        Command::Serve { bind } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring
                .telemetry
                .record("serve", serde_json::json!({ "bind": bind }))?;
            crate::server::serve(&bind)
        }
        Command::Backup { path } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("backup", serde_json::json!({}))?;
            let sessions_dir = crate::session::default_sessions_dir()?;
            let sessions = crate::session::Session::list(&sessions_dir)?;
            let mut session_data = serde_json::Map::new();
            for id in &sessions {
                if let Ok(s) = crate::session::Session::load(&sessions_dir, id) {
                    if let Ok(json) = s.export() {
                        session_data.insert(id.clone(), serde_json::Value::String(json));
                    }
                }
            }
            let memory_path = crate::memory::default_memory_path()?;
            let memory = crate::memory::Memory::load(memory_path)?;
            let backup = serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "created_at": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                "config": serde_json::to_value(&config)?,
                "sessions": session_data,
                "memory": serde_json::to_value(&memory)?,
            });
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, serde_json::to_string_pretty(&backup)?)?;
            println!(
                "backed up {} session(s) and memory to {}",
                sessions.len(),
                path.display()
            );
            Ok(())
        }
        Command::Restore { path } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("restore", serde_json::json!({}))?;
            let raw = std::fs::read_to_string(&path)?;
            let backup: serde_json::Value = serde_json::from_str(&raw)
                .map_err(|e| crate::error::Error::InvalidArgs(format!("bad backup: {e}")))?;
            // Restore memory.
            if let Some(mem) = backup.get("memory") {
                let memory_path = crate::memory::default_memory_path()?;
                if let Some(parent) = memory_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&memory_path, serde_json::to_string_pretty(mem)?)?;
            }
            // Restore sessions.
            if let Some(sessions) = backup
                .get("sessions")
                .and_then(serde_json::Value::as_object)
            {
                let sessions_dir = crate::session::default_sessions_dir()?;
                std::fs::create_dir_all(&sessions_dir)?;
                for (_id, val) in sessions {
                    if let Some(json) = val.as_str() {
                        if let Ok(s) = crate::session::Session::import(json) {
                            let _ = s.save(&sessions_dir);
                        }
                    }
                }
            }
            println!("restored from {}", path.display());
            Ok(())
        }
        Command::Token { text, file } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("token", serde_json::json!({}))?;
            let text = if let Some(f) = file {
                std::fs::read_to_string(&f)?
            } else {
                text.ok_or_else(|| {
                    crate::error::Error::InvalidArgs("provide text or --file <path>".into())
                })?
            };
            let count = crate::context::estimate_tokens(&text);
            println!("{count} tokens");
            Ok(())
        }
        Command::Hooks => {
            let config = Config::load()?;
            if config.hooks.is_empty() {
                println!("no hooks configured");
            } else {
                for h in &config.hooks {
                    let before = h.before.as_deref().unwrap_or("-");
                    let after = h.after.as_deref().unwrap_or("-");
                    println!("{}: before={before} after={after}", h.name);
                }
            }
            Ok(())
        }
        Command::Permission => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let registry = wiring.registry;
            for name in registry.names() {
                let declared = registry
                    .get(&name)
                    .map(|t| t.permission())
                    .unwrap_or(crate::permission::Permission::Allow);
                println!("{name}: {declared:?}");
            }
            Ok(())
        }
        Command::Git { action } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("git", serde_json::json!({}))?;
            let ws = config.workspace_root();
            match action {
                GitAction::Log {
                    limit,
                    stat,
                    author,
                    grep,
                    since,
                    all,
                } => {
                    let limit_str = limit.to_string();
                    let mut args: Vec<String> = vec![
                        "log".into(),
                        "--oneline".into(),
                        "-n".into(),
                        limit_str.clone(),
                    ];
                    if all {
                        args.push("--all".into());
                    }
                    if stat {
                        args.push("--stat".into());
                    }
                    if let Some(author) = author {
                        args.push("--author".into());
                        args.push(author);
                    }
                    if let Some(grep) = grep {
                        args.push("--grep".into());
                        args.push(grep);
                    }
                    if let Some(since) = since {
                        args.push("--since".into());
                        args.push(since);
                    }
                    let out = std::process::Command::new("git")
                        .args(&args)
                        .current_dir(&ws)
                        .output()?;
                    let text = String::from_utf8_lossy(&out.stdout).into_owned();
                    if text.trim().is_empty() {
                        println!("no commits");
                    } else {
                        println!("{text}");
                    }
                }
                GitAction::Branch => {
                    let out = std::process::Command::new("git")
                        .args(["branch", "-a"])
                        .current_dir(&ws)
                        .output()?;
                    println!("{}", String::from_utf8_lossy(&out.stdout));
                }
                GitAction::Remote { action } => match action {
                    RemoteAction::Show => {
                        let out = std::process::Command::new("git")
                            .args(["remote", "-v"])
                            .current_dir(&ws)
                            .output()?;
                        let text = String::from_utf8_lossy(&out.stdout).into_owned();
                        if text.trim().is_empty() {
                            println!("no remotes configured");
                        } else {
                            println!("{text}");
                        }
                    }
                    RemoteAction::Add { name, url } => {
                        let status = std::process::Command::new("git")
                            .args(["remote", "add", &name, &url])
                            .current_dir(&ws)
                            .status()?;
                        if !status.success() {
                            return Err(crate::error::Error::Tool(format!(
                                "git remote add {name} failed with {status}"
                            )));
                        }
                        println!("added remote {name}");
                    }
                    RemoteAction::Remove { name } => {
                        let status = std::process::Command::new("git")
                            .args(["remote", "remove", &name])
                            .current_dir(&ws)
                            .status()?;
                        if !status.success() {
                            return Err(crate::error::Error::Tool(format!(
                                "git remote remove {name} failed with {status}"
                            )));
                        }
                        println!("removed remote {name}");
                    }
                    RemoteAction::Rename { old, new } => {
                        let status = std::process::Command::new("git")
                            .args(["remote", "rename", &old, &new])
                            .current_dir(&ws)
                            .status()?;
                        if !status.success() {
                            return Err(crate::error::Error::Tool(format!(
                                "git remote rename {old} failed with {status}"
                            )));
                        }
                        println!("renamed remote {old} to {new}");
                    }
                },
                GitAction::Status => {
                    let out = std::process::Command::new("git")
                        .args(["status", "--short"])
                        .current_dir(&ws)
                        .output()?;
                    let text = String::from_utf8_lossy(&out.stdout).into_owned();
                    if text.trim().is_empty() {
                        println!("working tree clean");
                    } else {
                        println!("{text}");
                    }
                }
                GitAction::BranchCreate { name } => {
                    let status = std::process::Command::new("git")
                        .args(["checkout", "-b", &name])
                        .current_dir(&ws)
                        .status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git checkout -b {name} failed with {status}"
                        )));
                    }
                    println!("created and switched to branch {name}");
                }
                GitAction::Switch { name } => {
                    let status = std::process::Command::new("git")
                        .args(["checkout", &name])
                        .current_dir(&ws)
                        .status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git checkout {name} failed with {status}"
                        )));
                    }
                    println!("switched to branch {name}");
                }
                GitAction::Stash { action } => match action {
                    StashAction::List => {
                        let out = std::process::Command::new("git")
                            .args(["stash", "list"])
                            .current_dir(&ws)
                            .output()?;
                        let text = String::from_utf8_lossy(&out.stdout).into_owned();
                        if text.trim().is_empty() {
                            println!("no stashes");
                        } else {
                            println!("{text}");
                        }
                    }
                    StashAction::Push => {
                        let status = std::process::Command::new("git")
                            .args(["stash"])
                            .current_dir(&ws)
                            .status()?;
                        if !status.success() {
                            return Err(crate::error::Error::Tool(format!(
                                "git stash failed with {status}"
                            )));
                        }
                        println!("changes stashed");
                    }
                    StashAction::Pop => {
                        let status = std::process::Command::new("git")
                            .args(["stash", "pop"])
                            .current_dir(&ws)
                            .status()?;
                        if !status.success() {
                            return Err(crate::error::Error::Tool(format!(
                                "git stash pop failed with {status}"
                            )));
                        }
                        println!("stash restored");
                    }
                    StashAction::Apply { index } => {
                        let mut cmd = std::process::Command::new("git");
                        cmd.args(["stash", "apply"]).current_dir(&ws);
                        if let Some(idx) = index {
                            cmd.arg(format!("stash@{{{idx}}}"));
                        }
                        let status = cmd.status()?;
                        if !status.success() {
                            return Err(crate::error::Error::Tool(format!(
                                "git stash apply failed with {status}"
                            )));
                        }
                        println!("stash applied");
                    }
                },
                GitAction::Blame { file } => {
                    let out = std::process::Command::new("git")
                        .args(["blame", "--", &file])
                        .current_dir(&ws)
                        .output()?;
                    let text = String::from_utf8_lossy(&out.stdout).into_owned();
                    if text.trim().is_empty() {
                        println!("no blame info for {file}");
                    } else {
                        println!("{text}");
                    }
                }
                GitAction::Tag { action } => match action {
                    TagAction::List => {
                        let out = std::process::Command::new("git")
                            .args(["tag", "-l"])
                            .current_dir(&ws)
                            .output()?;
                        let text = String::from_utf8_lossy(&out.stdout).into_owned();
                        if text.trim().is_empty() {
                            println!("no tags");
                        } else {
                            println!("{text}");
                        }
                    }
                    TagAction::Create { name, message } => {
                        let mut cmd = std::process::Command::new("git");
                        cmd.args(["tag", "-a", &name]).current_dir(&ws);
                        if let Some(msg) = &message {
                            cmd.args(["-m", msg]);
                        }
                        let status = cmd.status()?;
                        if !status.success() {
                            return Err(crate::error::Error::Tool(format!(
                                "git tag {name} failed with {status}"
                            )));
                        }
                        println!("created tag {name}");
                    }
                    TagAction::Delete { name } => {
                        let status = std::process::Command::new("git")
                            .args(["tag", "-d", &name])
                            .current_dir(&ws)
                            .status()?;
                        if !status.success() {
                            return Err(crate::error::Error::Tool(format!(
                                "git tag -d {name} failed with {status}"
                            )));
                        }
                        println!("deleted tag {name}");
                    }
                },
                GitAction::Add { files } => {
                    if files.is_empty() {
                        return Err(crate::error::Error::InvalidArgs(
                            "at least one file required".into(),
                        ));
                    }
                    let status = std::process::Command::new("git")
                        .arg("add")
                        .args(&files)
                        .current_dir(&ws)
                        .status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git add failed with {status}"
                        )));
                    }
                    println!("staged {} file(s)", files.len());
                }
                GitAction::Commit { message, all } => {
                    let mut cmd = std::process::Command::new("git");
                    cmd.arg("commit").current_dir(&ws);
                    if all {
                        cmd.arg("-a");
                    }
                    match message {
                        Some(msg) => {
                            cmd.args(["-m", &msg]);
                        }
                        None => {
                            cmd.arg("--no-edit");
                        }
                    }
                    let status = cmd.status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git commit failed with {status}"
                        )));
                    }
                    println!("committed");
                }
                GitAction::Push {
                    remote,
                    branch,
                    force,
                } => {
                    let mut cmd = std::process::Command::new("git");
                    cmd.arg("push").current_dir(&ws);
                    if force {
                        cmd.arg("--force");
                    }
                    // `git push <remote> <branch>`: if a branch is given but no
                    // remote, default the remote to `origin`.
                    if branch.is_some() && remote.is_none() {
                        cmd.arg("origin");
                    }
                    if let Some(remote) = remote {
                        cmd.arg(&remote);
                    }
                    if let Some(branch) = branch {
                        cmd.arg(&branch);
                    }
                    let status = cmd.status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git push failed with {status}"
                        )));
                    }
                    println!("pushed");
                }
                GitAction::Pull => {
                    let status = std::process::Command::new("git")
                        .args(["pull"])
                        .current_dir(&ws)
                        .status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git pull failed with {status}"
                        )));
                    }
                    println!("pulled");
                }
                GitAction::Fetch => {
                    let status = std::process::Command::new("git")
                        .args(["fetch"])
                        .current_dir(&ws)
                        .status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git fetch failed with {status}"
                        )));
                    }
                    println!("fetched");
                }
                GitAction::Reset { soft, hard, commit } => {
                    let mut cmd = std::process::Command::new("git");
                    cmd.arg("reset").current_dir(&ws);
                    if soft {
                        cmd.arg("--soft");
                    } else if hard {
                        cmd.arg("--hard");
                    }
                    if let Some(commit) = commit {
                        cmd.arg(&commit);
                    }
                    let status = cmd.status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git reset failed with {status}"
                        )));
                    }
                    println!("reset done");
                }
                GitAction::Show { reference, stat } => {
                    let mut cmd = std::process::Command::new("git");
                    cmd.arg("show").current_dir(&ws);
                    if stat {
                        cmd.arg("--stat");
                    }
                    cmd.arg(&reference);
                    let out = cmd.output()?;
                    let text = String::from_utf8_lossy(&out.stdout).into_owned();
                    if text.trim().is_empty() {
                        println!("no output for {reference}");
                    } else {
                        println!("{text}");
                    }
                }
                GitAction::Merge { branch } => {
                    let status = std::process::Command::new("git")
                        .args(["merge", &branch])
                        .current_dir(&ws)
                        .status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git merge {branch} failed with {status}"
                        )));
                    }
                    println!("merged {branch}");
                }
                GitAction::Checkout { branch, reference } => {
                    let mut cmd = std::process::Command::new("git");
                    cmd.arg("checkout").current_dir(&ws);
                    if branch {
                        cmd.arg("-b");
                    }
                    cmd.arg(&reference);
                    let status = cmd.status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git checkout {reference} failed with {status}"
                        )));
                    }
                    if branch {
                        println!("created and checked out {reference}");
                    } else {
                        println!("checked out {reference}");
                    }
                }
                GitAction::Clean { force } => {
                    let mut cmd = std::process::Command::new("git");
                    cmd.arg("clean").current_dir(&ws);
                    if force {
                        cmd.arg("-f");
                    } else {
                        cmd.arg("-n");
                    }
                    let out = cmd.output()?;
                    let text = String::from_utf8_lossy(&out.stdout).into_owned();
                    if text.trim().is_empty() {
                        println!("nothing to clean");
                    } else {
                        println!("{text}");
                    }
                }
                GitAction::Diff {
                    staged,
                    stat,
                    name_only,
                } => {
                    let base: &[&str] = if staged {
                        &["diff", "--staged"]
                    } else {
                        &["diff", "HEAD"]
                    };
                    let mut args: Vec<&str> = base.to_vec();
                    if stat {
                        args.push("--stat");
                    }
                    if name_only {
                        args.push("--name-only");
                    }
                    let out = std::process::Command::new("git")
                        .args(&args)
                        .current_dir(&ws)
                        .output()?;
                    let text = String::from_utf8_lossy(&out.stdout).into_owned();
                    if text.trim().is_empty() {
                        println!("no changes");
                    } else {
                        println!("{text}");
                    }
                }
                GitAction::CherryPick { commit } => {
                    let status = std::process::Command::new("git")
                        .args(["cherry-pick", &commit])
                        .current_dir(&ws)
                        .status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git cherry-pick {commit} failed with {status}"
                        )));
                    }
                    println!("cherry-picked {commit}");
                }
                GitAction::Rebase { branch } => {
                    let status = std::process::Command::new("git")
                        .args(["rebase", &branch])
                        .current_dir(&ws)
                        .status()?;
                    if !status.success() {
                        return Err(crate::error::Error::Tool(format!(
                            "git rebase {branch} failed with {status}"
                        )));
                    }
                    println!("rebased onto {branch}");
                }
            }
            Ok(())
        }
        Command::Pr { base, title, body } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("pr", serde_json::json!({}))?;
            let ws = config.workspace_root();
            if which("gh") {
                let title = match title {
                    Some(t) => t,
                    None => {
                        let out = std::process::Command::new("git")
                            .args(["log", "-1", "--pretty=%s"])
                            .current_dir(&ws)
                            .output()?;
                        String::from_utf8_lossy(&out.stdout).trim().to_string()
                    }
                };
                let body = body.unwrap_or_else(|| "Generated by forge.".to_string());
                let mut cmd = std::process::Command::new("gh");
                cmd.args([
                    "pr", "create", "--base", &base, "--title", &title, "--body", &body,
                ])
                .current_dir(&ws);
                let status = cmd.status()?;
                if !status.success() {
                    return Err(crate::error::Error::Tool(format!(
                        "gh pr create failed with {status}"
                    )));
                }
            } else {
                return Err(crate::error::Error::Tool(
                    "gh CLI not found on PATH; install GitHub CLI to create PRs".into(),
                ));
            }
            Ok(())
        }
        Command::Sandbox { action } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            match action {
                SandboxAction::Run { command } => {
                    wiring
                        .telemetry
                        .record("sandbox", serde_json::json!({ "command": command }))?;
                    let sandbox = crate::sandbox::Sandbox::new(true);
                    let result = sandbox.run(&command)?;
                    println!("{}", result.output.trim_end());
                    if result.exit_code != 0 {
                        return Err(crate::error::Error::Tool(format!(
                            "sandbox command exited with {}",
                            result.exit_code
                        )));
                    }
                }
            }
            Ok(())
        }
        Command::Search { query } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring
                .telemetry
                .record("search", serde_json::json!({ "query": query }))?;
            let ctx = crate::tools::ToolContext::new(config.workspace_root());
            let tool = crate::tools::search::SearchTool::new();
            let result =
                crate::tools::Tool::run(&tool, &serde_json::json!({ "query": query }), &ctx)?;
            println!("{}", result.output);
            Ok(())
        }
        Command::Web { query } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring
                .telemetry
                .record("web", serde_json::json!({ "query": query }))?;
            let ctx = crate::tools::ToolContext::new(config.workspace_root());
            let tool = crate::tools::web_search::WebSearchTool::new();
            let result =
                crate::tools::Tool::run(&tool, &serde_json::json!({ "query": query }), &ctx)?;
            println!("{}", result.output);
            Ok(())
        }
        Command::Context { session } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("context", serde_json::json!({}))?;
            let dir = crate::session::default_sessions_dir()?;
            let s = crate::session::Session::load(&dir, &session)?;
            println!("session: {}", s.id);
            println!("messages: {}", s.messages.len());
            println!("tokens: {}", s.token_usage());
            Ok(())
        }
        Command::Model { action } => match action {
            ModelAction::Use { target } => {
                let path = crate::config::config_path()?;
                let mut config = Config::load()?;
                let provider = if let Ok(idx) = target.parse::<usize>() {
                    config.saved_providers.get(idx).cloned().ok_or_else(|| {
                        crate::error::Error::InvalidArgs(format!(
                            "no saved provider at index {idx}"
                        ))
                    })?
                } else {
                    config
                        .saved_providers
                        .iter()
                        .find(|p| p.model.as_deref() == Some(target.as_str()))
                        .cloned()
                        .ok_or_else(|| {
                            crate::error::Error::InvalidArgs(format!(
                                "no saved provider named {target}"
                            ))
                        })?
                };
                config.provider = provider;
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                println!(
                    "active model: {}",
                    config.provider.model.as_deref().unwrap_or("(none)")
                );
                Ok(())
            }
            ModelAction::List => {
                let config = Config::load()?;
                if config.saved_providers.is_empty() {
                    println!("no saved providers");
                } else {
                    for (idx, provider) in config.saved_providers.iter().enumerate() {
                        println!(
                            "{idx}: {} ({})",
                            provider.model.as_deref().unwrap_or("(no model)"),
                            provider.base_url.as_deref().unwrap_or("(default)")
                        );
                    }
                }
                Ok(())
            }
        },
        Command::Completions { shell } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring
                .telemetry
                .record("completions", serde_json::json!({ "shell": shell }))?;
            let script = match shell.as_str() {
                "bash" => completions_bash(),
                "zsh" => completions_zsh(),
                "fish" => completions_fish(),
                other => {
                    return Err(crate::error::Error::InvalidArgs(format!(
                        "unsupported shell {other}; use bash, zsh, or fish"
                    )))
                }
            };
            println!("{script}");
            Ok(())
        }
        Command::Man => {
            print_man();
            Ok(())
        }
        Command::Upgrade => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("upgrade", serde_json::json!({}))?;
            let ws = config.workspace_root();
            let pull = std::process::Command::new("git")
                .args(["pull", "--ff-only"])
                .current_dir(&ws)
                .output()?;
            if !pull.status.success() {
                return Err(crate::error::Error::Tool(format!(
                    "git pull failed: {}",
                    String::from_utf8_lossy(&pull.stderr)
                )));
            }
            println!("{}", String::from_utf8_lossy(&pull.stdout).trim_end());
            let build = std::process::Command::new("cargo")
                .args(["build", "--release"])
                .current_dir(&ws)
                .status()?;
            if !build.success() {
                return Err(crate::error::Error::Tool(format!(
                    "cargo build --release failed with {build}"
                )));
            }
            println!("forge upgraded and rebuilt");
            Ok(())
        }
        Command::Whoami => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("whoami", serde_json::json!({}))?;
            let user = std::env::var("USER").unwrap_or_else(|_| "unknown".into());
            let home = std::env::var("HOME").unwrap_or_else(|_| "unknown".into());
            println!("user: {user}");
            println!("home: {home}");
            println!(
                "model: {}",
                config.provider.model.as_deref().unwrap_or("(none)")
            );
            println!(
                "base_url: {}",
                config.provider.base_url.as_deref().unwrap_or("(default)")
            );
            println!(
                "api_key: {}",
                if config.provider.api_key.is_some() {
                    "set"
                } else {
                    "unset"
                }
            );
            println!("workspace: {}", config.workspace_root().display());
            println!("telemetry: {}", if config.telemetry { "on" } else { "off" });
            Ok(())
        }
        Command::Init { template } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring
                .telemetry
                .record("init", serde_json::json!({ "template": template }))?;
            if let Some(tpl) = template {
                scaffold_template(&tpl)?;
                return Ok(());
            }
            let path = crate::config::config_path()?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let sample = r#"{
  "workspace": ".",
  "provider": {
    "base_url": "https://api.openai.com/v1",
    "model": "gpt-4o-mini",
    "api_key": ""
  },
  "max_turns": 10,
  "mcp_servers": [],
  "plugins_dir": ".forge/plugins",
  "hooks": [],
  "telemetry": true
}
"#;
            std::fs::write(&path, sample)?;
            println!("wrote sample config to {}", path.display());

            // Create a minimal project scaffold in the current directory.
            let cwd = std::env::current_dir()?;
            let readme = cwd.join("README.md");
            if !readme.exists() {
                std::fs::write(&readme, "# My Project\n\nA project managed with forge.\n")?;
                println!("wrote {}", readme.display());
            }
            let src = cwd.join("src");
            std::fs::create_dir_all(&src)?;
            let main = src.join("main.rs");
            if !main.exists() {
                std::fs::write(
                    &main,
                    "fn main() {\n    println!(\"hello from forge\");\n}\n",
                )?;
                println!("wrote {}", main.display());
            }
            let forge_dir = cwd.join(".forge");
            std::fs::create_dir_all(&forge_dir)?;
            println!("scaffolded project in {}", cwd.display());
            Ok(())
        }
        Command::Repl => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("repl", serde_json::json!({}))?;
            run_repl(&config)?;
            Ok(())
        }
        Command::Ssh { host, command } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring
                .telemetry
                .record("ssh", serde_json::json!({ "host": host }))?;
            let ctx = crate::tools::ToolContext::new(config.workspace_root());
            let tool = crate::tools::ssh::SshTool::new();
            let result = crate::tools::Tool::run(
                &tool,
                &serde_json::json!({ "host": host, "command": command }),
                &ctx,
            )?;
            println!("{}", result.output);
            Ok(())
        }
        Command::Terminal { command } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("terminal", serde_json::json!({}))?;
            let ctx = crate::tools::ToolContext::new(config.workspace_root());
            let tool = crate::tools::terminal::TerminalTool::new();
            let result =
                crate::tools::Tool::run(&tool, &serde_json::json!({ "command": command }), &ctx)?;
            println!("{}", result.output);
            Ok(())
        }
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Run {
            prompt,
            file,
            resume,
            notify,
            workspace,
            max_turns,
            watch,
        } => {
            let mut config = Config::load()?;
            if let Some(ws) = workspace {
                config.workspace = Some(ws);
            }
            let prompt = if let Some(file) = file {
                std::fs::read_to_string(&file)?
            } else {
                prompt
            };
            let turns = max_turns.or(config.max_turns).unwrap_or(10);
            let provider = HttpProvider::new(&config.provider)?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let agent =
                Agent::new(Box::new(provider), wiring.registry, turns).with_hooks(wiring.hooks);
            let dir = crate::session::default_sessions_dir()?;
            let mut session = if let Some(id) = resume {
                crate::session::Session::load(&dir, &id)?
            } else {
                let id = format!(
                    "sess-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                );
                crate::session::Session::new(&id)
            };
            let outcome = agent.run_into(&mut session.messages, &prompt)?;
            session.save(&dir)?;
            wiring.telemetry.record(
                "run",
                serde_json::json!({
                    "turns": outcome.turns,
                    "tool_calls": outcome.tool_calls,
                }),
            )?;
            if notify {
                crate::notify::Notifier::new(true)
                    .notify("forge", &format!("run finished ({} turns)", outcome.turns))?;
            }
            println!("{}", outcome.final_text);
            eprintln!(
                "[forge] {} turn(s), {} tool call(s), session {}",
                outcome.turns, outcome.tool_calls, session.id
            );
            if watch {
                let ws = config.workspace_root();
                println!(
                    "watching {} — re-running on change (Ctrl+C to stop)",
                    ws.display()
                );
                let mut last: std::collections::HashMap<std::path::PathBuf, std::time::SystemTime> =
                    std::collections::HashMap::new();
                loop {
                    let mut changed = false;
                    for entry in walkdir::WalkDir::new(&ws)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        if entry.file_type().is_file() {
                            if let Ok(meta) = entry.metadata() {
                                if let Ok(modified) = meta.modified() {
                                    if last
                                        .get(entry.path())
                                        .map(|t| *t != modified)
                                        .unwrap_or(true)
                                    {
                                        last.insert(entry.path().to_path_buf(), modified);
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                    if changed {
                        let mut session = crate::session::Session::new(format!(
                            "sess-{}",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0)
                        ));
                        let outcome = agent.run_into(&mut session.messages, &prompt)?;
                        session.save(&dir)?;
                        println!("{}", outcome.final_text);
                        eprintln!(
                            "[forge] {} turn(s), {} tool call(s), session {}",
                            outcome.turns, outcome.tool_calls, session.id
                        );
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
            Ok(())
        }
    }
}
