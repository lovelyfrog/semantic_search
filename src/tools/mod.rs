//! CLI / batch tooling entry points used by this crate.

pub mod cli;
pub mod mcp;
pub mod mcp_registry;
pub mod service;

pub use cli::{Cli, run_cli};
