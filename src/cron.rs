//! Cron / automations: a scheduler that runs jobs on a fixed interval.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// A scheduled job: runs a shell command every `interval`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub name: String,
    pub interval_secs: u64,
    pub command: String,
}

/// A scheduler that runs jobs on their intervals.
#[derive(Debug, Clone, Default)]
pub struct Scheduler {
    jobs: Vec<Job>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, job: Job) {
        self.jobs.push(job);
    }

    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    /// Run every job once now, returning each job's output.
    pub fn run_once(&self) -> Result<Vec<(String, String)>> {
        let mut results = Vec::new();
        for job in &self.jobs {
            let output = run_command(&job.command)?;
            results.push((job.name.clone(), output));
        }
        Ok(results)
    }

    /// Run the scheduler forever: each job runs immediately, then on its
    /// interval. Blocks the calling thread.
    pub fn run_forever(&self) -> Result<()> {
        let jobs = Arc::new(self.jobs.clone());
        let mut handles = Vec::new();
        for job in jobs.iter() {
            let job = job.clone();
            handles.push(std::thread::spawn(move || loop {
                let _ = run_command(&job.command);
                std::thread::sleep(Duration::from_secs(job.interval_secs));
            }));
        }
        for handle in handles {
            let _ = handle.join();
        }
        Ok(())
    }
}

/// Run a shell command and return its output.
fn run_command(command: &str) -> Result<String> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| Error::Agent(format!("run job: {e}")))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_once_executes_jobs() {
        let mut scheduler = Scheduler::new();
        scheduler.add(Job {
            name: "echo".into(),
            interval_secs: 60,
            command: "echo hello".into(),
        });
        let results = scheduler.run_once().unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].1.contains("hello"));
    }

    #[test]
    fn empty_scheduler_runs_once() {
        let scheduler = Scheduler::new();
        assert!(scheduler.run_once().unwrap().is_empty());
    }
}
