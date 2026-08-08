//! The plan executor: runs a typed plan's DAG under a budget, with observable
//! progress and truthful outcomes. A plan that exhausts its budget reports
//! `Partial`; a plan with a failed task reports `Failed`; only a fully
//! successful plan reports `Completed`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use crate::agent::Agent;
use crate::context::estimate_tokens;
use crate::error::{Error, Result};
use crate::plan::{Plan, PlanTask};

/// The terminal status of a plan run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStatus {
    Completed,
    Partial,
    Failed,
}

/// The outcome of a plan run.
#[derive(Debug, Clone)]
pub struct PlanOutcome {
    pub status: PlanStatus,
    pub results: HashMap<String, String>,
    pub tasks_run: usize,
    pub tokens_used: usize,
}

/// A progress callback: `(task_id, status)` where status is "started" or
/// "completed".
pub type ProgressFn = dyn Fn(&str, &str) + Send + Sync;

/// Runs a plan's DAG under its budget.
pub struct PlanRunner {
    agent: Arc<Agent>,
    progress: Option<Box<ProgressFn>>,
}

impl PlanRunner {
    pub fn new(agent: Agent) -> Self {
        Self {
            agent: Arc::new(agent),
            progress: None,
        }
    }

    /// Attach a progress callback for observability.
    pub fn with_progress(mut self, progress: Box<ProgressFn>) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Execute the plan. Returns a truthful outcome.
    pub fn run(&self, plan: &Plan) -> Result<PlanOutcome> {
        plan.validate()?;

        let started = Instant::now();
        let mut results: HashMap<String, String> = HashMap::new();
        let mut tokens_used = 0usize;
        let mut remaining: HashSet<String> = plan.tasks.iter().map(|t| t.id.clone()).collect();
        let mut done: HashSet<String> = HashSet::new();
        let mut failed: HashSet<String> = HashSet::new();
        let mut tasks_run = 0usize;
        let mut budget_exhausted = false;

        while !remaining.is_empty() {
            // Wall-clock budget.
            if let Some(max_wall) = plan.budget.max_wall_secs {
                if started.elapsed().as_secs() > max_wall {
                    budget_exhausted = true;
                    break;
                }
            }

            let ready: Vec<&PlanTask> = plan
                .tasks
                .iter()
                .filter(|t| {
                    remaining.contains(&t.id) && t.depends_on.iter().all(|d| done.contains(d))
                })
                .collect();
            if ready.is_empty() {
                return Err(Error::Agent("plan stalled: no ready tasks".into()));
            }

            let batch: Vec<&PlanTask> = ready.into_iter().take(plan.budget.max_workers).collect();
            let mut handles = Vec::new();
            for task in &batch {
                let agent = Arc::clone(&self.agent);
                let prompt = task.prompt.clone();
                let id = task.id.clone();
                handles.push(std::thread::spawn(move || {
                    let text = agent
                        .run_prompt(&prompt)
                        .unwrap_or_else(|e| format!("error: {e}"));
                    (id, text, estimate_tokens(&prompt))
                }));
            }
            for handle in handles {
                let (id, text, toks) = handle
                    .join()
                    .map_err(|_| Error::Agent("task panicked".into()))?;
                if let Some(progress) = &self.progress {
                    progress(&id, "completed");
                }
                // A task whose result is an error is a failed task.
                if text.starts_with("error:") {
                    failed.insert(id.clone());
                }
                results.insert(id.clone(), text);
                tokens_used += toks;
                remaining.remove(&id);
                done.insert(id);
                tasks_run += 1;
            }

            if tokens_used > plan.budget.max_tokens {
                budget_exhausted = true;
                break;
            }
        }

        // Truthful status: budget exhaustion is Partial, a failed task is Failed,
        // only full success is Completed.
        let status = if budget_exhausted {
            PlanStatus::Partial
        } else if !failed.is_empty() {
            PlanStatus::Failed
        } else if remaining.is_empty() {
            PlanStatus::Completed
        } else {
            PlanStatus::Partial
        };

        Ok(PlanOutcome {
            status,
            results,
            tasks_run,
            tokens_used,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::mock::{MockProvider, ScriptTurn};
    use crate::plan::{Plan, PlanBudget, PlanTask};
    use crate::tools::Registry;

    fn plan_with(tasks: Vec<PlanTask>, max_tokens: usize) -> Plan {
        Plan {
            name: "t".into(),
            description: "d".into(),
            budget: PlanBudget {
                max_workers: 2,
                max_tokens,
                max_wall_secs: None,
            },
            tasks,
        }
    }

    fn agent_answering(n: usize) -> Agent {
        let script = (0..n)
            .map(|i| ScriptTurn::Answer(format!("r{i}")))
            .collect();
        Agent::new(Box::new(MockProvider::new(script)), Registry::builtin(), 5)
    }

    #[test]
    fn completes_a_dag() {
        let plan = plan_with(
            vec![
                PlanTask {
                    id: "a".into(),
                    prompt: "a".into(),
                    depends_on: vec![],
                    tools: vec![],
                    phase: None,
                },
                PlanTask {
                    id: "b".into(),
                    prompt: "b".into(),
                    depends_on: vec!["a".into()],
                    tools: vec![],
                    phase: None,
                },
            ],
            100_000,
        );
        let outcome = PlanRunner::new(agent_answering(2)).run(&plan).unwrap();
        assert_eq!(outcome.status, PlanStatus::Completed);
        assert_eq!(outcome.tasks_run, 2);
    }

    #[test]
    fn budget_exhaustion_is_partial() {
        let plan = plan_with(
            vec![PlanTask {
                id: "a".into(),
                prompt: "a fairly long prompt that costs more than one token".into(),
                depends_on: vec![],
                tools: vec![],
                phase: None,
            }],
            1, // tiny budget
        );
        let outcome = PlanRunner::new(agent_answering(1)).run(&plan).unwrap();
        assert_eq!(outcome.status, PlanStatus::Partial);
    }

    #[test]
    fn failed_task_reports_failed() {
        use crate::agent::mock::call;
        use serde_json::json;
        // A provider that keeps calling tools so the agent hits max turns and
        // the task errors.
        let agent = Agent::new(
            Box::new(MockProvider::new(vec![ScriptTurn::Tools(vec![call(
                "c1",
                "glob",
                json!({"pattern": "*"}),
            )])])),
            Registry::builtin(),
            1,
        );
        let plan = plan_with(
            vec![PlanTask {
                id: "a".into(),
                prompt: "a".into(),
                depends_on: vec![],
                tools: vec![],
                phase: None,
            }],
            100_000,
        );
        let outcome = PlanRunner::new(agent).run(&plan).unwrap();
        assert_eq!(outcome.status, PlanStatus::Failed);
    }
}
