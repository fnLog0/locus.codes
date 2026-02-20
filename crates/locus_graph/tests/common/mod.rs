//! Common test utilities and configuration.

use locus_graph::{LocusGraphClient, LocusGraphConfig};
use std::path::PathBuf;

/// Load .env file from project root if available.
fn load_dotenv() {
    // Try to find .env in current dir or parent directories
    let mut path = std::env::current_dir().unwrap();
    loop {
        let env_path = path.join(".env");
        if env_path.exists() {
            let _ = dotenvy::from_path(&env_path);
            return;
        }
        if !path.pop() {
            return;
        }
    }
}

/// GRPC endpoint for tests. Defaults to localhost, override via GRPC_ENDPOINT env var.
pub fn grpc_url() -> String {
    load_dotenv();
    std::env::var("GRPC_ENDPOINT")
        .or_else(|_| std::env::var("LOCUSGRAPH_SERVER_URL"))
        .unwrap_or_else(|_| "http://localhost:50051".to_string())
}

/// Get agent secret from env (required).
pub fn agent_secret() -> String {
    load_dotenv();
    std::env::var("LOCUSGRAPH_AGENT_SECRET")
        .expect("LOCUSGRAPH_AGENT_SECRET must be set in .env or environment")
}

/// Get graph ID from env, with fallback.
pub fn graph_id() -> String {
    load_dotenv();
    std::env::var("LOCUSGRAPH_GRAPH_ID")
        .unwrap_or_else(|_| "locus-agent".to_string())
}

/// Create a test client with configuration from .env.
#[allow(dead_code)]
pub async fn test_client() -> LocusGraphClient {
    load_dotenv();
    let config = LocusGraphConfig::new(grpc_url(), agent_secret(), graph_id())
        .db_path(PathBuf::from("/tmp/locus_graph_test.db"))
        .cache_reads(false) // Disable cache for tests to get fresh data
        .queue_stores(false); // Disable queue for immediate writes in tests

    LocusGraphClient::new(config)
        .await
        .expect("Failed to create test client")
}

/// Generate a unique context ID for testing.
#[allow(dead_code)]
pub fn unique_context_id(prefix: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{}:{}", prefix, timestamp)
}
