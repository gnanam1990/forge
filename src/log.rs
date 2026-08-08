//! A small structured logger. Logs to stderr with a level and a timestamp, and
//! can be silenced. Kept dependency-free.

use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Log levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

impl Level {
    fn tag(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
        }
    }
}

static MIN_LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);

/// Set the minimum level that is emitted.
pub fn set_level(level: Level) {
    MIN_LEVEL.store(level as u8, Ordering::Relaxed);
}

/// Emit a log line if `level` is at or above the configured minimum.
pub fn log(level: Level, message: &str) {
    if (level as u8) > MIN_LEVEL.load(Ordering::Relaxed) {
        return;
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    eprintln!("[{secs}] [{}] {message}", level.tag());
}

pub fn error(message: &str) {
    log(Level::Error, message);
}
pub fn warn(message: &str) {
    log(Level::Warn, message);
}
pub fn info(message: &str) {
    log(Level::Info, message);
}
pub fn debug(message: &str) {
    log(Level::Debug, message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_ordered() {
        assert!(Level::Error < Level::Warn);
        assert!(Level::Warn < Level::Info);
        assert!(Level::Info < Level::Debug);
    }

    #[test]
    fn set_level_accepts_all() {
        for level in [Level::Error, Level::Warn, Level::Info, Level::Debug] {
            set_level(level);
            log(level, "test");
        }
    }
}
