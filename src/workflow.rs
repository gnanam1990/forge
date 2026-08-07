//! The workflow engine: a DAG of tasks with phases, validated and executed in
//! topological order with bounded parallel fan-out and a token budget.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::agent::Agent;
use crate::context::estimate_tokens;
use crate::error::{Error, Result};
use crate::watchdog::Watchdog;

/// A single task in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub phase: Option<String>,
}

/// A workflow: a named DAG of tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub tasks: Vec<Task>,
    #[serde(default = "default_workers")]
    pub max_workers: usize,
    #[serde(default = "default_tokens")]
    pub max_tokens: usize,
}

fn default_workers() -> usize {
    4
}
fn default_tokens() -> usize {
    100_000
}

impl Workflow {
    /// Validate the DAG: unique ids, resolved dependencies, no cycles.
    pub fn validate(&self) -> Result<()> {
        let mut ids = HashSet::new();
        for task in &self.tasks {
            if !ids.insert(task.id.clone()) {
                return Err(Error::InvalidArgs(format!(
                    "duplicate task id: {}",
                    task.id
                )));
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
                "workflow contains a dependency cycle".into(),
            ));
        }
        Ok(())
    }
}

/// The result of a completed workflow run.
#[derive(Debug, Clone)]
pub struct WorkflowOutcome {
    pub results: HashMap<String, String>,
    pub tasks_run: usize,
    pub tokens_used: usize,
}

/// Runs a workflow by dispatching each task to a sub-agent, respecting
/// dependencies, a worker bound, and a token budget.
pub struct WorkflowRunner {
    agent: Arc<Agent>,
    watchdog: Option<Watchdog>,
}

impl WorkflowRunner {
    pub fn new(agent: Agent) -> Self {
        Self {
            agent: Arc::new(agent),
            watchdog: None,
        }
    }

    /// Enable stall detection and retry for each task.
    pub fn with_watchdog(mut self, watchdog: Watchdog) -> Self {
        self.watchdog = Some(watchdog);
        self
    }

    /// Execute the workflow. Returns per-task results keyed by task id.
    pub fn run(&self, workflow: &Workflow) -> Result<WorkflowOutcome> {
        workflow.validate()?;

        let mut results: HashMap<String, String> = HashMap::new();
        let mut tokens_used = 0usize;
        let mut remaining: HashSet<String> = workflow.tasks.iter().map(|t| t.id.clone()).collect();
        let mut done: HashSet<String> = HashSet::new();
        let mut tasks_run = 0usize;

        while !remaining.is_empty() {
            // Find tasks whose dependencies are all satisfied.
            let ready: Vec<&Task> = workflow
                .tasks
                .iter()
                .filter(|t| {
                    remaining.contains(&t.id) && t.depends_on.iter().all(|d| done.contains(d))
                })
                .collect();

            if ready.is_empty() {
                return Err(Error::Agent("workflow stalled: no ready tasks".into()));
            }

            // Run the ready batch in parallel, bounded by max_workers.
            let batch: Vec<&Task> = ready.into_iter().take(workflow.max_workers).collect();
            let mut handles = Vec::new();
            for task in &batch {
                let agent = Arc::clone(&self.agent);
                let prompt = task.prompt.clone();
                let prompt_for_watchdog = prompt.clone();
                let id = task.id.clone();
                let watchdog = self.watchdog.clone();
                handles.push(std::thread::spawn(move || {
                    let text = match watchdog {
                        Some(w) => w
                            .run(move || agent.run_prompt(&prompt_for_watchdog))
                            .unwrap_or_else(|e| format!("error: {e}")),
                        None => agent
                            .run_prompt(&prompt)
                            .unwrap_or_else(|e| format!("error: {e}")),
                    };
                    (id, text, estimate_tokens(&prompt))
                }));
            }
            for handle in handles {
                let (id, text, toks) = handle
                    .join()
                    .map_err(|_| Error::Agent("task panicked".into()))?;
                results.insert(id, text);
                tokens_used += toks;
            }

            // Mark the batch done.
            for task in &batch {
                remaining.remove(&task.id);
                done.insert(task.id.clone());
                tasks_run += 1;
            }

            if tokens_used > workflow.max_tokens {
                return Err(Error::Agent("workflow exceeded token budget".into()));
            }
        }

        Ok(WorkflowOutcome {
            results,
            tasks_run,
            tokens_used,
        })
    }
}
