//! Command-line interface for forge.

use std::io::Write;
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
        /// Read the prompt from a file instead of the argument.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Override the workspace root.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Override the max number of turns.
        #[arg(long)]
        max_turns: Option<usize>,
    },
    /// Print the version.
    Version,
    /// List the built-in model catalog.
    Models,
    /// Show or set the provider: `provider [show|set <model>]`.
    Provider {
        /// Action: show | set.
        action: Option<String>,
        /// Model to set (for `set`).
        model: Option<String>,
    },
    /// Print a short usage guide.
    Docs,
    /// Show usage stats from telemetry.
    Stats,
    /// Show environment information.
    Env,
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
    Doctor,
    /// Print version, tools, and config summary.
    Info,
    /// Print the current config (with the API key redacted).
    Config,
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
    /// Write a sample config file to the default location.
    Init,
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
         memory <remember|recall|list>                 manage memory\n\
         cron <file> [--forever]                       run scheduled jobs\n\
         review                                        review the git diff\n\
         mcp <server> <tool> <args>                    call an MCP tool\n\
         browser <url> [--eval] [--click] [--type] [--screenshot]  browser\n\
         desktop <screenshot|click|type>               desktop control\n\
         plugin <list|enable|disable|add>              manage plugins\n\
         config                                        show config\n\
         doctor                                        check environment\n\
         info                                          show summary\n\
         setup                                         write config + samples\n\
         init                                          scaffold a project\n\
         version                                       print version\n\
         help                                          this help"
    );
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
        Command::Chat => {
            let config = Config::load()?;
            crate::tui::run_chat(&config)
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
        Command::Mcp { server, tool, args } => {
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
        Command::Browser {
            url,
            eval,
            click,
            r#type,
            screenshot,
        } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            let browser = crate::browser::Browser::launch()?;
            let target = browser.open(&url)?;
            wiring
                .telemetry
                .record("browser", serde_json::json!({ "url": url }))?;
            println!("opened {url} (target {})", target.id);
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
        Command::Doctor => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("doctor", serde_json::json!({}))?;
            let mut ok = true;
            let mut check = |name: &str, pass: bool, detail: &str| {
                println!("[{}] {name}: {detail}", if pass { "ok" } else { "MISSING" });
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
            if ok {
                println!("\nall checks passed");
            } else {
                println!("\nsome checks failed — see above");
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
        Command::Config => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("config", serde_json::json!({}))?;
            let mut redacted = config.clone();
            if let Some(key) = redacted.provider.api_key.as_mut() {
                if !key.is_empty() {
                    *key = "***".to_string();
                }
            }
            println!("{}", serde_json::to_string_pretty(&redacted)?);
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
        Command::Provider { action, model } => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            match action.as_deref().unwrap_or("show") {
                "show" => {
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
                "set" => {
                    let model = model
                        .ok_or_else(|| crate::error::Error::InvalidArgs("model required".into()))?;
                    let path = crate::config::config_path()?;
                    let mut config = config;
                    config.provider.model = Some(model.clone());
                    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                    println!("set model to {model}");
                }
                "list" => {
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
                "add" => {
                    let model = model
                        .ok_or_else(|| crate::error::Error::InvalidArgs("model required".into()))?;
                    let path = crate::config::config_path()?;
                    let mut config = config;
                    config.saved_providers.push(crate::config::ProviderConfig {
                        model: Some(model.clone()),
                        ..Default::default()
                    });
                    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
                    println!("added provider {model}");
                }
                other => {
                    return Err(crate::error::Error::InvalidArgs(format!(
                        "unknown provider action {other}"
                    )))
                }
            }
            wiring
                .telemetry
                .record("provider", serde_json::json!({ "action": action }))?;
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
        Command::Env => {
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
        Command::Init => {
            let config = Config::load()?;
            let wiring = crate::wiring::build_wiring(&config)?;
            wiring.telemetry.record("init", serde_json::json!({}))?;
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
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Run {
            prompt,
            file,
            workspace,
            max_turns,
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
            wiring.telemetry.record(
                "run",
                serde_json::json!({
                    "turns": outcome.turns,
                    "tool_calls": outcome.tool_calls,
                }),
            )?;
            println!("{}", outcome.final_text);
            eprintln!(
                "[forge] {} turn(s), {} tool call(s), session {id}",
                outcome.turns, outcome.tool_calls
            );
            Ok(())
        }
    }
}
