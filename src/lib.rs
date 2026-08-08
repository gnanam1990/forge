//! forge — an original, self-contained coding agent written in Rust.
//!
//! The crate is split into a small library (`forge`) and a thin binary
//! (`src/main.rs`). The library exposes the agent loop, the tool system, and
//! configuration so the whole thing is testable without a network or a model.

pub mod agent;
pub mod browser;
pub mod cli;
pub mod config;
pub mod context;
pub mod cron;
pub mod desktop;
pub mod error;
pub mod hooks;
pub mod log;
pub mod mcp;
pub mod memory;
pub mod notify;
pub mod permission;
pub mod plan;
pub mod plan_exec;
pub mod plugin;
pub mod posture;
pub mod review;
pub mod sandbox;
pub mod session;
pub mod telemetry;
pub mod tools;
pub mod tui;
pub mod watchdog;
pub mod workflow;

pub use error::{Error, Result};
