//! Configuration loading and resolution.
//!
//! Supports TOML config files, environment variables, and CLI arguments
//! with a well-defined priority chain.

pub mod loader;

pub use loader::{load_config, resolve_graph_path, ServerConfig};
