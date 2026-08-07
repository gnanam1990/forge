//! Permission and approval: tools declare a safety level, a policy can allow or
//! deny tools by name, and the agent loop consults both before running a tool.

/// The safety level a tool declares, and the outcome of a policy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// Run without asking.
    Allow,
    /// Ask the user before running.
    Prompt,
    /// Never run.
    Deny,
}

/// A policy of allow/deny rules keyed by tool name. Deny wins over allow; a
/// rule wins over the tool's declared default.
#[derive(Debug, Clone, Default)]
pub struct Policy {
    allow: Vec<String>,
    deny: Vec<String>,
}

impl Policy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allow a tool by name.
    pub fn allow(mut self, tool: &str) -> Self {
        self.allow.push(tool.to_string());
        self
    }

    /// Deny a tool by name.
    pub fn deny(mut self, tool: &str) -> Self {
        self.deny.push(tool.to_string());
        self
    }

    /// Decide the effective permission for a tool given its declared default.
    pub fn decide(&self, tool: &str, default: Permission) -> Permission {
        if self.deny.iter().any(|t| t == tool) {
            return Permission::Deny;
        }
        if self.allow.iter().any(|t| t == tool) {
            return Permission::Allow;
        }
        default
    }
}

/// An approver resolves a prompt decision to a yes/no. The default (None) denies
/// in headless runs; an interactive surface supplies one.
pub type Approver = dyn Fn(&str) -> bool + Send + Sync;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_wins_over_allow() {
        let policy = Policy::new().allow("bash").deny("bash");
        assert_eq!(policy.decide("bash", Permission::Prompt), Permission::Deny);
    }

    #[test]
    fn allow_overrides_prompt_default() {
        let policy = Policy::new().allow("bash");
        assert_eq!(policy.decide("bash", Permission::Prompt), Permission::Allow);
    }

    #[test]
    fn no_rule_uses_default() {
        let policy = Policy::new();
        assert_eq!(
            policy.decide("read_file", Permission::Allow),
            Permission::Allow
        );
        assert_eq!(
            policy.decide("bash", Permission::Prompt),
            Permission::Prompt
        );
    }
}
