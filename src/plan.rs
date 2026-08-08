//! The typed plan model, matching the zeromaxing PRD: a plan is structured data
//! (never an executed script) with a name, description, tasks, and a budget.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The read-only tools a plan task may use. Plan tasks are read-only by design
/// (PRD §8): granting mutation authority from a plan is an authority widening
/// that needs its own approval surface.
pub const READ_ONLY_TOOLS: &[&str] = &[
    "read_file",
    "list_directory",
    "glob",
    "grep",
    "search",
    "git_status",
    "git_diff",
    "web_fetch",
    "ask_user",
];

/// A single plan task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTask {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub phase: Option<String>,
}

/// The plan budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanBudget {
    pub max_workers: usize,
    pub max_tokens: usize,
    #[serde(default)]
    pub max_wall_secs: Option<u64>,
}

/// A typed plan: a DAG of read-only tasks under a budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub name: String,
    pub description: String,
    pub tasks: Vec<PlanTask>,
    pub budget: PlanBudget,
}

impl Plan {
    /// Validate the plan: unique ids, resolved dependencies, no cycles, a
    /// required budget, and read-only tools only.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::InvalidArgs("plan name is required".into()));
        }
        if self.budget.max_tokens == 0 {
            return Err(Error::InvalidArgs("budget.max_tokens is required".into()));
        }
        if self.budget.max_workers == 0 {
            return Err(Error::InvalidArgs("budget.max_workers must be >= 1".into()));
        }

        let mut ids = HashSet::new();
        for task in &self.tasks {
            if !ids.insert(task.id.clone()) {
                return Err(Error::InvalidArgs(format!(
                    "duplicate task id: {}",
                    task.id
                )));
            }
            for tool in &task.tools {
                if !READ_ONLY_TOOLS.contains(&tool.as_str()) {
                    return Err(Error::InvalidArgs(format!(
                        "task {} requests non-read-only tool {}",
                        task.id, tool
                    )));
                }
            }
        }
        for task in &self.tasks {
            for dep in &task.depends_on {
                if !ids.contains(dep) {
                    return Err(Error::InvalidArgs(format!(
                        "task {} depends on unknown task {}",
                        task.id, dep
                    )));
                }
                if dep == &task.id {
                    return Err(Error::InvalidArgs(format!(
                        "task {} depends on itself",
                        task.id
                    )));
                }
            }
        }

        // Cycle detection via Kahn's algorithm.
        let mut indegree: HashMap<&str, usize> = HashMap::new();
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
        for task in &self.tasks {
            indegree.entry(&task.id).or_insert(0);
            for dep in &task.depends_on {
                *indegree.entry(&task.id).or_insert(0) += 1;
                dependents.entry(dep.as_str()).or_default().push(&task.id);
            }
        }
        let mut queue: Vec<&str> = indegree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut visited = 0usize;
        while let Some(id) = queue.pop() {
            visited += 1;
            if let Some(deps) = dependents.get(id) {
                for dep in deps {
                    let entry = indegree.get_mut(dep).unwrap();
                    *entry -= 1;
                    if *entry == 0 {
                        queue.push(dep);
                    }
                }
            }
        }
        if visited != self.tasks.len() {
            return Err(Error::InvalidArgs(
                "plan contains a dependency cycle".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_plan() -> Plan {
        Plan {
            name: "test".into(),
            description: "a test plan".into(),
            budget: PlanBudget {
                max_workers: 2,
                max_tokens: 100_000,
                max_wall_secs: None,
            },
            tasks: vec![PlanTask {
                id: "a".into(),
                prompt: "do a".into(),
                depends_on: vec![],
                tools: vec!["read_file".into()],
                phase: None,
            }],
        }
    }

    #[test]
    fn valid_plan_passes() {
        assert!(valid_plan().validate().is_ok());
    }

    #[test]
    fn rejects_write_tool() {
        let mut plan = valid_plan();
        plan.tasks[0].tools = vec!["write_file".into()];
        let err = plan.validate().unwrap_err();
        assert!(err.to_string().contains("non-read-only"));
    }

    #[test]
    fn rejects_cycle() {
        let mut plan = valid_plan();
        plan.tasks.push(PlanTask {
            id: "b".into(),
            prompt: "b".into(),
            depends_on: vec!["a".into()],
            tools: vec![],
            phase: None,
        });
        plan.tasks[0].depends_on = vec!["b".into()];
        assert!(plan.validate().is_err());
    }

    #[test]
    fn rejects_missing_budget() {
        let mut plan = valid_plan();
        plan.budget.max_tokens = 0;
        assert!(plan.validate().is_err());
    }
}
