//! Plugins: a named bundle of tools and hooks that can be registered into the
//! agent. Plugins let third parties extend forge without touching core. Plugins
//! can be built in code or loaded from a directory of JSON files.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::hooks::HookDispatcher;
use crate::tools::{Registry, Tool, ToolContext, ToolResult};

/// A plugin: a name plus the tools and hooks it contributes.
pub struct Plugin {
    pub name: String,
    pub tools: Vec<Box<dyn Tool>>,
    pub hooks: HookDispatcher,
}

impl Plugin {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tools: Vec::new(),
            hooks: HookDispatcher::new(),
        }
    }

    /// Add a tool to the plugin.
    pub fn tool(mut self, tool: Box<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Register the plugin's tools into a registry, consuming the plugin.
    pub fn register_into(self, registry: &mut Registry) {
        for tool in self.tools {
            registry.register(tool);
        }
    }
}

/// The on-disk schema for a plugin file.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginFile {
    pub name: String,
    #[serde(default)]
    pub tools: Vec<PluginToolDef>,
}

/// A command-backed tool definition in a plugin file.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginToolDef {
    pub name: String,
    pub command: String,
    pub description: String,
}

/// A tool that runs a shell command.
struct CommandTool {
    name: String,
    command: String,
    description: String,
}

impl Tool for CommandTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn run(
        &self,
        args: &serde_json::Value,
        _ctx: &ToolContext,
    ) -> crate::error::Result<ToolResult> {
        let args_json = serde_json::to_string(args)?;
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("{} {}", self.command, args_json))
            .output()?;
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            Ok(ToolResult::ok(text))
        } else {
            Ok(ToolResult::err(format!(
                "command exited with {}\n{text}",
                output.status
            )))
        }
    }
}

/// Load all `*.json` plugin files from a directory and register their tools.
pub fn load_plugins_from_dir(dir: &Path, registry: &mut Registry) -> crate::error::Result<usize> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut count = 0usize;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        // Skip the registry's own state file.
        if path.file_name().and_then(|n| n.to_str()) == Some("state.json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path)?;
        let plugin: PluginFile = serde_json::from_str(&raw)
            .map_err(|e| crate::error::Error::Config(format!("parse {}: {e}", path.display())))?;
        for tool in plugin.tools {
            registry.register(Box::new(CommandTool {
                name: tool.name,
                command: tool.command,
                description: tool.description,
            }));
            count += 1;
        }
    }
    Ok(count)
}

/// A tracked plugin entry in the registry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginEntry {
    pub name: String,
    pub enabled: bool,
    pub tools: Vec<String>,
}

/// A registry that tracks plugins, their enabled state, and their tools. State
/// is persisted so enable/disable survives restarts.
#[derive(Debug, Clone, Default)]
pub struct PluginRegistry {
    plugins: Vec<PluginEntry>,
    state_path: PathBuf,
}

impl PluginRegistry {
    pub fn new(state_path: PathBuf) -> Self {
        let mut registry = Self {
            plugins: Vec::new(),
            state_path,
        };
        registry.load_state();
        registry
    }

    /// Load plugin files from a directory, respecting the persisted enabled
    /// state. Returns the number of tools registered.
    pub fn load_dir(&mut self, dir: &Path) -> crate::error::Result<usize> {
        if !dir.is_dir() {
            return Ok(0);
        }
        let mut count = 0usize;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            // Skip the registry's own state file.
            if path.file_name().and_then(|n| n.to_str()) == Some("state.json") {
                continue;
            }
            let raw = std::fs::read_to_string(&path)?;
            let plugin: PluginFile = serde_json::from_str(&raw).map_err(|e| {
                crate::error::Error::Config(format!("parse {}: {e}", path.display()))
            })?;
            let enabled = self
                .plugins
                .iter()
                .find(|p| p.name == plugin.name)
                .map(|p| p.enabled)
                .unwrap_or(true);
            let tools: Vec<String> = plugin.tools.iter().map(|t| t.name.clone()).collect();
            self.plugins.retain(|p| p.name != plugin.name);
            self.plugins.push(PluginEntry {
                name: plugin.name,
                enabled,
                tools: tools.clone(),
            });
            count += tools.len();
        }
        self.save_state();
        Ok(count)
    }

    /// Register the enabled plugins' tools into a registry.
    pub fn register_into(&self, registry: &mut Registry, dir: &Path) -> crate::error::Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            // Skip the registry's own state file.
            if path.file_name().and_then(|n| n.to_str()) == Some("state.json") {
                continue;
            }
            let raw = std::fs::read_to_string(&path)?;
            let plugin: PluginFile = serde_json::from_str(&raw).map_err(|e| {
                crate::error::Error::Config(format!("parse {}: {e}", path.display()))
            })?;
            let enabled = self
                .plugins
                .iter()
                .find(|p| p.name == plugin.name)
                .map(|p| p.enabled)
                .unwrap_or(true);
            if !enabled {
                continue;
            }
            for tool in plugin.tools {
                registry.register(Box::new(CommandTool {
                    name: tool.name,
                    command: tool.command,
                    description: tool.description,
                }));
            }
        }
        Ok(())
    }

    /// List the tracked plugins.
    pub fn list(&self) -> &[PluginEntry] {
        &self.plugins
    }

    /// Enable a plugin by name.
    pub fn enable(&mut self, name: &str) -> crate::error::Result<()> {
        let entry = self
            .plugins
            .iter_mut()
            .find(|p| p.name == name)
            .ok_or_else(|| crate::error::Error::Config(format!("unknown plugin {name}")))?;
        entry.enabled = true;
        self.save_state();
        Ok(())
    }

    /// Disable a plugin by name.
    pub fn disable(&mut self, name: &str) -> crate::error::Result<()> {
        let entry = self
            .plugins
            .iter_mut()
            .find(|p| p.name == name)
            .ok_or_else(|| crate::error::Error::Config(format!("unknown plugin {name}")))?;
        entry.enabled = false;
        self.save_state();
        Ok(())
    }

    fn load_state(&mut self) {
        if let Ok(raw) = std::fs::read_to_string(&self.state_path) {
            if let Ok(plugins) = serde_json::from_str::<Vec<PluginEntry>>(&raw) {
                self.plugins = plugins;
            }
        }
    }

    fn save_state(&self) {
        if let Some(parent) = self.state_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(raw) = serde_json::to_string(&self.plugins) {
            let _ = std::fs::write(&self.state_path, raw);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{ToolContext, ToolResult};
    use serde_json::json;

    struct EchoTool;
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "echo a value"
        }
        fn run(
            &self,
            args: &serde_json::Value,
            _ctx: &ToolContext,
        ) -> crate::error::Result<ToolResult> {
            Ok(ToolResult::ok(
                args.get("value")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
            ))
        }
    }

    #[test]
    fn plugin_registers_tools() {
        let plugin = Plugin::new("demo").tool(Box::new(EchoTool));
        let mut registry = Registry::builtin();
        plugin.register_into(&mut registry);
        assert!(registry.get("echo").is_some());
        let ctx = ToolContext::new(std::env::temp_dir());
        let res = registry
            .get("echo")
            .unwrap()
            .run(&json!({"value": "hi"}), &ctx)
            .unwrap();
        assert_eq!(res.output, "hi");
    }

    #[test]
    fn load_dir_skips_state_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("demo.json"),
            r#"{"name":"demo","tools":[{"name":"greet","command":"echo","description":"greets"}]}"#,
        )
        .unwrap();
        // A state.json that is NOT a valid plugin file must be ignored.
        std::fs::write(
            dir.path().join("state.json"),
            r#"[{"name":"demo","enabled":true,"tools":["greet"]}]"#,
        )
        .unwrap();
        let mut registry = PluginRegistry::new(dir.path().join("state.json"));
        let count = registry.load_dir(dir.path()).unwrap();
        assert_eq!(count, 1);
        assert_eq!(registry.list().len(), 1);
        assert_eq!(registry.list()[0].name, "demo");
    }
}
