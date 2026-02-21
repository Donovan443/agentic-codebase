//! Multi-tenant session registry — lazy-loads per-user graph files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::server::McpServer;
use crate::AcbResult;

/// Mutex type selected by feature: `tokio::sync::Mutex` for SSE,
/// `std::sync::Mutex` when tokio is unavailable.
#[cfg(feature = "sse")]
type MutexType<T> = tokio::sync::Mutex<T>;
#[cfg(not(feature = "sse"))]
type MutexType<T> = std::sync::Mutex<T>;

/// Registry of per-user MCP server instances for multi-tenant mode.
pub struct TenantRegistry {
    data_dir: PathBuf,
    servers: HashMap<String, Arc<MutexType<McpServer>>>,
}

impl TenantRegistry {
    /// Create a new tenant registry backed by the given data directory.
    pub fn new(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            servers: HashMap::new(),
        }
    }

    /// Get or create an MCP server for the given user ID.
    ///
    /// On first access, loads `{data_dir}/{user_id}.acb` and creates
    /// a server with the graph pre-loaded.
    pub fn get_or_create(&mut self, user_id: &str) -> AcbResult<Arc<MutexType<McpServer>>> {
        if let Some(server) = self.servers.get(user_id) {
            return Ok(server.clone());
        }

        // Ensure data directory exists
        std::fs::create_dir_all(&self.data_dir).map_err(|e| {
            crate::AcbError::Io(std::io::Error::other(format!(
                "Failed to create data dir {}: {e}",
                self.data_dir.display()
            )))
        })?;

        let graph_path = self.data_dir.join(format!("{user_id}.acb"));

        tracing::info!(
            "Opening graph for user '{user_id}': {}",
            graph_path.display()
        );

        let mut server = McpServer::new();

        // If the graph file exists, load it
        if graph_path.is_file() {
            match crate::AcbReader::read_from_file(&graph_path) {
                Ok(graph) => {
                    server.load_graph(user_id.to_string(), graph);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load graph for user '{user_id}': {e} — starting empty"
                    );
                }
            }
        }

        let server = Arc::new(MutexType::new(server));
        self.servers.insert(user_id.to_string(), server.clone());

        Ok(server)
    }

    /// Number of active tenant sessions.
    pub fn count(&self) -> usize {
        self.servers.len()
    }
}
