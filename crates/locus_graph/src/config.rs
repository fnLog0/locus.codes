//! Configuration for LocusGraph client.
//!
//! One `graph_id` for the entire system, set once at startup.

use std::path::PathBuf;

/// Configuration for connecting to LocusGraph.
#[derive(Clone, Debug)]
pub struct LocusGraphConfig {
    /// gRPC server endpoint (e.g. "http://127.0.0.1:50051")
    pub grpc_endpoint: String,
    /// Agent secret token for Authorization
    pub agent_secret: String,
    /// Graph ID — single brain, never per-session
    pub graph_id: String,
    /// Path to SQLite DB (cache + event queue)
    pub db_path: PathBuf,
    /// Whether to use cache for read operations
    pub cache_reads: bool,
    /// Whether to queue store_event and send in background
    pub queue_stores: bool,
}

impl LocusGraphConfig {
    /// Create config from environment variables.
    ///
    /// Required: `LOCUSGRAPH_AGENT_SECRET`
    /// Optional: `LOCUSGRAPH_SERVER_URL` (default: http://127.0.0.1:50051)
    /// Optional: `LOCUSGRAPH_GRAPH_ID` (default: locus-agent)
    pub fn from_env() -> Result<Self, crate::error::LocusGraphError> {
        let agent_secret = std::env::var("LOCUSGRAPH_AGENT_SECRET")
            .map_err(|_| {
                crate::error::LocusGraphError::Config(
                    "LOCUSGRAPH_AGENT_SECRET not set".into(),
                )
            })?;

        let grpc_endpoint = std::env::var("LOCUSGRAPH_SERVER_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());

        let graph_id =
            std::env::var("LOCUSGRAPH_GRAPH_ID").unwrap_or_else(|_| "locus-agent".to_string());

        let db_path = std::env::temp_dir().join("locus_graph_cache.db");

        Ok(Self {
            grpc_endpoint,
            agent_secret,
            graph_id,
            db_path,
            cache_reads: true,
            queue_stores: true,
        })
    }

    /// Create a new config with all required fields.
    pub fn new(
        grpc_endpoint: impl Into<String>,
        agent_secret: impl Into<String>,
        graph_id: impl Into<String>,
    ) -> Self {
        Self {
            grpc_endpoint: grpc_endpoint.into(),
            agent_secret: agent_secret.into(),
            graph_id: graph_id.into(),
            db_path: std::env::temp_dir().join("locus_graph_cache.db"),
            cache_reads: true,
            queue_stores: true,
        }
    }

    /// Set custom database path for cache and queue.
    pub fn db_path(mut self, path: PathBuf) -> Self {
        self.db_path = path;
        self
    }

    /// Enable or disable read caching.
    pub fn cache_reads(mut self, on: bool) -> Self {
        self.cache_reads = on;
        self
    }

    /// Enable or disable write queueing.
    pub fn queue_stores(mut self, on: bool) -> Self {
        self.queue_stores = on;
        self
    }
}
