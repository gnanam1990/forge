//! Plugins: a named bundle of tools and hooks that can be registered into the
//! agent. Plugins let third parties extend forge without touching core.

use crate::hooks::HookDispatcher;
use crate::tools::{Registry, Tool};

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
}
