//! Configuration for LocusGraph client.
//!
//! One `graph_id` for the entire system, set once at startup.

use std::path::{Path, PathBuf};

/// Default path for cache/queue DB when not overridden by LOCUSGRAPH_DB_PATH.
/// Prefers repo .locus when running inside a git repo (walk up from cwd for .git),
/// else ~/.locus/locus_graph_cache.db, else temp dir.
pub fn default_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("LOCUSGRAPH_DB_PATH") {
        let path = PathBuf::from(p);
        if path.is_absolute() {
            return path;
        }
        // Relative path: resolve from cwd
        if let Ok(cwd) = std::env::current_dir() {
            return cwd.join(path);
        }
        return path;
    }
    // Prefer project .locus when inside a git repo
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(repo_root) = find_repo_root(&cwd) {
            return repo_root.join(".locus").join("locus_graph_cache.db");
        }
    }
    dirs::home_dir()
        .map(|h| h.join(".locus").join("locus_graph_cache.db"))
        .unwrap_or_else(|| std::env::temp_dir().join("locus_graph_cache.db"))
}

/// Walk up from `dir` and return the first directory that contains `.git`.
fn find_repo_root(mut dir: &Path) -> Option<PathBuf> {
    loop {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

/// Configuration for connecting to LocusGraph.
#[derive(Clone, Debug)]
pub struct LocusGraphConfig {
    /// gRPC server endpoint (e.g. "https://grpc-dev.locusgraph.com:443")
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
    /// Optional: `LOCUSGRAPH_SERVER_URL` (default: https://grpc-dev.locusgraph.com:443)
    /// Optional: `LOCUSGRAPH_GRAPH_ID` (default: locus-agent)
    pub fn from_env() -> Result<Self, crate::error::LocusGraphError> {
        let agent_secret = std::env::var("LOCUSGRAPH_AGENT_SECRET")
            .map_err(|_| {
                crate::error::LocusGraphError::Config(
                    "LOCUSGRAPH_AGENT_SECRET not set".into(),
                )
            })?;

        let grpc_endpoint = std::env::var("LOCUSGRAPH_SERVER_URL")
            .unwrap_or_else(|_| "https://grpc-dev.locusgraph.com:443".to_string());

        let graph_id =
            std::env::var("LOCUSGRAPH_GRAPH_ID").unwrap_or_else(|_| "locus-agent".to_string());

        let db_path = default_db_path();

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
            db_path: default_db_path(),
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
