//! forge — an original, self-contained coding agent written in Rust.
//!
//! The crate is split into a small library (`forge`) and a thin binary
//! (`src/main.rs`). The library exposes the agent loop, the tool system, and
//! configuration so the whole thing is testable without a network or a model.

pub mod agent;
pub mod cli;
pub mod config;
pub mod error;
pub mod tools;

pub use error::{Error, Result};
