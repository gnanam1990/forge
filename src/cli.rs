//! Command-line interface for forge.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

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
        /// Override the workspace root.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Override the max number of turns.
        #[arg(long)]
        max_turns: Option<usize>,
    },
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
    /// Start an interactive chat session.
    Chat,
    /// Manage cross-session memory: `remember <key> <value>`, `recall <key>`,
    /// `list`.
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
    /// Connect to an MCP server and call a tool: `mcp <server> <tool> <json-args>`.
    Mcp {
        /// The MCP server command to spawn.
        server: String,
        /// The tool to call.
        tool: String,
        /// JSON arguments for the tool.
        args: String,
    },
    /// Launch a headless browser and open a URL.
    Browser {
        /// The URL to open.
        url: String,
    },
    /// Write a sample config file to the default location.
    Init,
}

/// Run the CLI and return a process exit code.
pub fn run(cli: Cli) -> i32 {
    match dispatch(cli) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Tools => {
            let registry = Registry::builtin();
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
            let raw = std::fs::read_to_string(&file)?;
            let prompts: Vec<String> = serde_json::from_str(&raw).map_err(|e| {
                crate::error::Error::InvalidArgs(format!("parse {}: {e}", file.display()))
            })?;
            let agent = Agent::new(Box::new(provider), Registry::builtin(), turns);
            let results = agent.run_parallel(&prompts)?;
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
            let agent = Agent::new(Box::new(provider), Registry::builtin(), turns);
            let runner = crate::workflow::WorkflowRunner::new(agent);
            let outcome = runner.run(&workflow)?;
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
            let dir = crate::session::default_sessions_dir()?;
            let mut session = crate::session::Session::load(&dir, &session)?;
            let agent = Agent::new(Box::new(provider), Registry::builtin(), turns);
            let outcome = agent.run_into(&mut session.messages, &prompt)?;
            session.save(&dir)?;
            println!("{}", outcome.final_text);
            eprintln!(
                "[forge] {} turn(s), {} tool call(s)",
                outcome.turns, outcome.tool_calls
            );
            Ok(())
        }
        Command::Sessions => {
            let dir = crate::session::default_sessions_dir()?;
            let ids = crate::session::Session::list(&dir)?;
            if ids.is_empty() {
                println!("no saved sessions");
            } else {
                for id in ids {
                    println!("{id}");
                }
            }
            Ok(())
        }
        Command::Chat => {
            let config = Config::load()?;
            crate::tui::run_chat(&config)
        }
        Command::Memory { action, key, value } => {
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
                other => {
                    return Err(crate::error::Error::InvalidArgs(format!(
                        "unknown action {other}"
                    )))
                }
            }
            Ok(())
        }
        Command::Cron { file, forever } => {
            let raw = std::fs::read_to_string(&file)?;
            let jobs: Vec<crate::cron::Job> = serde_json::from_str(&raw).map_err(|e| {
                crate::error::Error::InvalidArgs(format!("parse {}: {e}", file.display()))
            })?;
            let mut scheduler = crate::cron::Scheduler::new();
            for job in jobs {
                scheduler.add(job);
            }
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
            let workspace = Config::load()?.workspace_root();
            let diff = std::process::Command::new("git")
                .args(["diff"])
                .current_dir(&workspace)
                .output()
                .map_err(|e| crate::error::Error::Agent(format!("git diff: {e}")))?;
            let diff_text = String::from_utf8_lossy(&diff.stdout).into_owned();
            let review = crate::review::review_diff(&diff_text);
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
        Command::Mcp { server, tool, args } => {
            let mut client = crate::mcp::McpClient::connect(&server, &[])?;
            let tools = client.list_tools()?;
            eprintln!("[forge] MCP tools: {}", tools.join(", "));
            let args: serde_json::Value = serde_json::from_str(&args)
                .map_err(|e| crate::error::Error::InvalidArgs(format!("bad args: {e}")))?;
            let output = client.call_tool(&tool, args)?;
            println!("{output}");
            Ok(())
        }
        Command::Browser { url } => {
            let browser = crate::browser::Browser::launch()?;
            let target = browser.open(&url)?;
            println!("opened {url} (target {target})");
            for tab in browser.list()? {
                println!("tab: {tab}");
            }
            Ok(())
        }
        Command::Init => {
            let path = crate::config::config_path()?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let sample = r#"{
  "workspace": "/path/to/your/project",
  "provider": {
    "base_url": "https://api.openai.com/v1",
    "model": "gpt-4o-mini",
    "api_key": ""
  },
  "max_turns": 10
}
"#;
            std::fs::write(&path, sample)?;
            println!("wrote sample config to {}", path.display());
            Ok(())
        }
        Command::Run {
            prompt,
            workspace,
            max_turns,
        } => {
            let mut config = Config::load()?;
            if let Some(ws) = workspace {
                config.workspace = Some(ws);
            }
            let turns = max_turns.or(config.max_turns).unwrap_or(10);
            let provider = HttpProvider::new(&config.provider)?;
            let registry = Registry::builtin();
            let agent = Agent::new(Box::new(provider), registry, turns);
            let dir = crate::session::default_sessions_dir()?;
            let id = format!(
                "sess-{}",
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
                "[forge] {} turn(s), {} tool call(s), session {id}",
                outcome.turns, outcome.tool_calls
            );
            Ok(())
        }
    }
}
