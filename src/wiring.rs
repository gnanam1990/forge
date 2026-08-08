//! Config-driven wiring: builds the tool registry, hook dispatcher, and
//! telemetry from a `Config`, auto-loading MCP servers, plugins, and hooks.

use crate::config::Config;
use crate::error::Result;
use crate::hooks::{HookContext, HookDispatcher};
use crate::mcp;
use crate::plugin;
use crate::telemetry::{default_telemetry_path, Telemetry};
use crate::tools::Registry;

/// The assembled runtime wiring for a run.
pub struct Wiring {
    pub registry: Registry,
    pub hooks: HookDispatcher,
    pub telemetry: Telemetry,
}

/// Build the wiring from a config.
pub fn build_wiring(config: &Config) -> Result<Wiring> {
    let mut registry = Registry::builtin();

    // Auto-register MCP servers.
    for server in &config.mcp_servers {
        let args: Vec<&str> = server.args.iter().map(String::as_str).collect();
        mcp::register_mcp_server(&mut registry, &server.command, &args)?;
    }

    // Load plugins from a directory.
    if let Some(dir) = &config.plugins_dir {
        plugin::load_plugins_from_dir(dir, &mut registry)?;
    }

    // Install hooks from config.
    let mut hooks = HookDispatcher::new();
    for hook in &config.hooks {
        if let Some(cmd) = &hook.before {
            let cmd = cmd.clone();
            hooks.add_before(hook.name.clone(), Box::new(move |_| run_hook_command(&cmd)));
        }
        if let Some(cmd) = &hook.after {
            let cmd = cmd.clone();
            hooks.add_after(hook.name.clone(), Box::new(move |_| run_hook_command(&cmd)));
        }
    }

    let telemetry = Telemetry::new(config.telemetry, default_telemetry_path()?);

    Ok(Wiring {
        registry,
        hooks,
        telemetry,
    })
}

/// Run a hook shell command.
fn run_hook_command(command: &str) -> Result<()> {
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(crate::error::Error::Tool(format!(
            "hook command exited with {status}"
        )))
    }
}

/// A no-op hook context helper for tests.
#[allow(dead_code)]
fn hook_ctx(tool: &str) -> HookContext {
    HookContext {
        tool: tool.into(),
        args: serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_builds_default_wiring() {
        let wiring = build_wiring(&Config::default()).unwrap();
        assert!(wiring.registry.get("read_file").is_some());
        assert!(wiring.hooks.is_empty());
    }

    #[test]
    fn config_hooks_are_installed() {
        let mut config = Config::default();
        config.hooks.push(crate::config::HookConfig {
            name: "log".into(),
            before: Some("true".into()),
            after: None,
        });
        let wiring = build_wiring(&config).unwrap();
        assert!(!wiring.hooks.is_empty());
    }
}
