//! Hooks: named callbacks that run before and after tool calls. A dispatcher
//! holds the hooks and runs them at the right points in the agent loop.

use serde_json::Value;

use crate::error::Result;

/// Context passed to a hook.
#[derive(Debug, Clone)]
pub struct HookContext {
    pub tool: String,
    pub args: Value,
}

/// A hook function.
pub type HookFn = dyn Fn(&HookContext) -> Result<()> + Send + Sync;

/// A named hook.
pub struct Hook {
    pub name: String,
    pub f: Box<HookFn>,
}

/// Runs before and after hooks around tool calls.
#[derive(Default)]
pub struct HookDispatcher {
    before: Vec<Hook>,
    after: Vec<Hook>,
}

impl HookDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a hook that runs before a tool call.
    pub fn add_before(&mut self, name: impl Into<String>, f: Box<HookFn>) {
        self.before.push(Hook {
            name: name.into(),
            f,
        });
    }

    /// Register a hook that runs after a tool call.
    pub fn add_after(&mut self, name: impl Into<String>, f: Box<HookFn>) {
        self.after.push(Hook {
            name: name.into(),
            f,
        });
    }

    /// Run all before-hooks. A hook error aborts the tool call.
    pub fn run_before(&self, ctx: &HookContext) -> Result<()> {
        for hook in &self.before {
            (hook.f)(ctx)?;
        }
        Ok(())
    }

    /// Run all after-hooks. Errors are collected but do not abort.
    pub fn run_after(&self, ctx: &HookContext) {
        for hook in &self.after {
            let _ = (hook.f)(ctx);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.before.is_empty() && self.after.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn hooks_run_in_order() {
        let mut dispatcher = HookDispatcher::new();
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let before_order = std::sync::Arc::clone(&order);
        dispatcher.add_before(
            "b",
            Box::new(move |_| {
                before_order.lock().unwrap().push("before");
                Ok(())
            }),
        );
        let after_order = std::sync::Arc::clone(&order);
        dispatcher.add_after(
            "a",
            Box::new(move |_| {
                after_order.lock().unwrap().push("after");
                Ok(())
            }),
        );
        let ctx = HookContext {
            tool: "bash".into(),
            args: Value::Null,
        };
        dispatcher.run_before(&ctx).unwrap();
        dispatcher.run_after(&ctx);
        assert_eq!(*order.lock().unwrap(), vec!["before", "after"]);
    }

    #[test]
    fn before_hook_can_block() {
        let mut dispatcher = HookDispatcher::new();
        dispatcher.add_before(
            "block",
            Box::new(|_| Err(crate::error::Error::Tool("blocked by hook".into()))),
        );
        let ctx = HookContext {
            tool: "bash".into(),
            args: Value::Null,
        };
        assert!(dispatcher.run_before(&ctx).is_err());
    }

    #[test]
    fn empty_dispatcher_is_noop() {
        let dispatcher = HookDispatcher::new();
        assert!(dispatcher.is_empty());
        let ctx = HookContext {
            tool: "x".into(),
            args: Value::Null,
        };
        assert!(dispatcher.run_before(&ctx).is_ok());
    }
}
