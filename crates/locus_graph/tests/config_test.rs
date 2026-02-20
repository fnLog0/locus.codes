//! Test that environment configuration is loaded correctly.
//!
//! Run with: cargo test -p locus-graph --test config_test

mod common;

use common::{agent_secret, graph_id, grpc_url};

#[test]
fn test_env_vars_loaded() {
    // These should load from .env file
    let secret = agent_secret();
    let graph = graph_id();
    let endpoint = grpc_url();

    println!("LOCUSGRAPH_AGENT_SECRET: {}...", &secret[..10.min(secret.len())]);
    println!("LOCUSGRAPH_GRAPH_ID: {}", graph);
    println!("GRPC_ENDPOINT: {}", endpoint);

    // Verify values from .env file
    assert!(!secret.is_empty(), "LOCUSGRAPH_AGENT_SECRET should be set");
    assert!(graph.starts_with("graph_"), "GRAPH_ID should start with 'graph_'");
    assert_eq!(endpoint, "https://grpc-dev.locusgraph.com:443");
}

#[test]
fn test_grpc_endpoint_override() {
    // If GRPC_ENDPOINT is set, it should take priority
    std::env::set_var("GRPC_ENDPOINT", "http://localhost:9999");
    let url = grpc_url();
    assert_eq!(url, "http://localhost:9999");
    std::env::remove_var("GRPC_ENDPOINT");
}
