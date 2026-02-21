//! Configuration loading from file, environment, and CLI arguments.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::AcbResult;

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Path to the .acb graph file.
    pub graph_path: String,
    /// Transport type ("stdio" or "sse").
    #[serde(default = "default_transport")]
    pub transport: String,
    /// SSE listen address (only used when transport is "sse").
    #[serde(default = "default_sse_addr")]
    pub sse_addr: String,
    /// Log level.
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_transport() -> String {
    "stdio".to_string()
}

fn default_sse_addr() -> String {
    "127.0.0.1:3000".to_string()
}

fn default_log_level() -> String {
    "warn".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            graph_path: resolve_default_graph_path(),
            transport: default_transport(),
            sse_addr: default_sse_addr(),
            log_level: default_log_level(),
        }
    }
}

/// Load configuration from a TOML file.
pub fn load_config(path: &str) -> AcbResult<ServerConfig> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        crate::AcbError::Io(std::io::Error::other(format!(
            "Failed to read config file {path}: {e}"
        )))
    })?;

    toml::from_str(&content).map_err(|e| {
        crate::AcbError::Io(std::io::Error::other(format!(
            "Failed to parse config: {e}"
        )))
    })
}

/// Resolve the graph file path using priority order:
///
/// 1. Explicit path (CLI arg)
/// 2. `ACB_GRAPH` environment variable
/// 3. `.acb/graph.acb` in current directory
/// 4. `~/.agentic-codebase/graph.acb` (global default)
pub fn resolve_graph_path(explicit: Option<&str>) -> String {
    if let Some(path) = explicit {
        return path.to_string();
    }

    if let Ok(env_path) = std::env::var("ACB_GRAPH") {
        return env_path;
    }

    let cwd_graph = PathBuf::from(".acb/graph.acb");
    if cwd_graph.exists() {
        return cwd_graph.display().to_string();
    }

    resolve_default_graph_path()
}

fn resolve_default_graph_path() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    format!("{home}/.agentic-codebase/graph.acb")
}
