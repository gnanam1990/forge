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
        Command::Browser {
            url,
            eval,
            click,
            r#type,
            screenshot,
        } => {
            let browser = crate::browser::Browser::launch()?;
            let target = browser.open(&url)?;
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
            let desktop = crate::desktop::Desktop::new();
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
            let mut ok = true;
            let mut check = |name: &str, pass: bool, detail: &str| {
                println!("[{}] {name}: {detail}", if pass { "ok" } else { "MISSING" });
                if !pass {
                    ok = false;
                }
            };
            let config = Config::load()?;
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
            let registry = Registry::builtin();
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
        Command::Effort { level } => {
            let effort = crate::posture::Effort::parse(&level).ok_or_else(|| {
                crate::error::Error::InvalidArgs(format!(
                    "unknown effort {level}; expected auto, balanced, thorough, or zeromaxing"
                ))
            })?;
            let posture = crate::posture::Posture::from_effort(effort);
            let auto = crate::posture::Posture::from_effort(crate::posture::Effort::Auto);
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
