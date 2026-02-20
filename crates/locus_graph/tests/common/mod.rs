//! Common test utilities and configuration.

use locus_graph::{LocusGraphClient, LocusGraphConfig};
use std::path::PathBuf;

pub const GRPC_URL: &str = "http://localhost:50051";
pub const AGENT_SECRET: &str = "5f5338d638bea5cfe65ec08ec03ab0058b8c9cbe4c2a16cbe1391d97c425df91";
pub const GRAPH_ID: &str = "graph_1a66bced-1cf2-49bd-a964-6d7ca2f40162";

/// Create a test client with the standard test configuration.
pub async fn test_client() -> LocusGraphClient {
    let config = LocusGraphConfig::new(GRPC_URL, AGENT_SECRET, GRAPH_ID)
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
