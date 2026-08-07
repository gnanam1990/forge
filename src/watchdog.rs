//! A stall watchdog: runs a task with a timeout and retries it a bounded number
//! of times when it makes no progress. A task that returns a real error is not
//! retried — only a stall (no completion within the timeout) is.

use std::sync::Arc;
use std::time::Duration;

use crate::error::{Error, Result};

/// Detects stalled tasks and retries them within a bound.
#[derive(Debug, Clone)]
pub struct Watchdog {
    timeout: Duration,
    max_retries: usize,
}

impl Watchdog {
    pub fn new(timeout: Duration, max_retries: usize) -> Self {
        Self {
            timeout,
            max_retries,
        }
    }

    /// Run a task function with stall detection. Returns the task's text on
    /// success, or an error if the task returns a real error or exhausts its
    /// retries on stalls.
    pub fn run<F>(&self, f: F) -> Result<String>
    where
        F: Fn() -> Result<String> + Send + Sync + 'static,
    {
        let f = Arc::new(f);
        let mut last_error: Option<Error> = None;
        for attempt in 0..=self.max_retries {
            let f = Arc::clone(&f);
            let (tx, rx) = std::sync::mpsc::channel();
            let handle = std::thread::spawn(move || {
                let result = f();
                let _ = tx.send(result);
            });
            match rx.recv_timeout(self.timeout) {
                Ok(Ok(text)) => return Ok(text),
                Ok(Err(e)) => {
                    // A real error is an answer, not a stall — do not retry.
                    last_error = Some(e);
                    break;
                }
                Err(_) => {
                    // Timed out: a stall. The thread may still be running; we
                    // cannot kill it, so it is left to finish in the background.
                    let _ = handle;
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            Error::Agent(format!(
                "task stalled after {} attempt(s)",
                self.max_retries + 1
            ))
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_quick_result() {
        let watchdog = Watchdog::new(Duration::from_millis(200), 1);
        let result = watchdog.run(|| Ok("done".into())).unwrap();
        assert_eq!(result, "done");
    }

    #[test]
    fn retries_on_stall() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let calls = Arc::new(AtomicUsize::new(0));
        let watchdog = Watchdog::new(Duration::from_millis(50), 2);
        let calls_inner = Arc::clone(&calls);
        let result = watchdog
            .run(move || {
                let n = calls_inner.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    std::thread::sleep(Duration::from_millis(500)); // stall
                    Ok("late".into())
                } else {
                    Ok("ok".into())
                }
            })
            .unwrap();
        assert_eq!(result, "ok");
        assert!(calls.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn real_error_is_not_retried() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let calls = Arc::new(AtomicUsize::new(0));
        let watchdog = Watchdog::new(Duration::from_millis(200), 3);
        let calls_inner = Arc::clone(&calls);
        let result = watchdog
            .run(move || {
                calls_inner.fetch_add(1, Ordering::SeqCst);
                Err(Error::Tool("boom".into()))
            })
            .unwrap_err();
        assert!(result.to_string().contains("boom"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
