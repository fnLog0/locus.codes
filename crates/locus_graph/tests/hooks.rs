//! Tests for LocusGraph client hooks.
//!
//! These tests require a running LocusGraph gRPC server at localhost:50051.

mod common;

use common::test_client;
use serde_json::json;

#[tokio::test]
async fn test_store_tool_run_success() {
    let client = test_client().await;

    client
        .store_tool_run(
            "bash",
            &json!({"command": "cargo build"}),
            &json!({"exit_code": 0, "output": "Build succeeded"}),
            1500,
            false,
        )
        .await;
}

#[tokio::test]
async fn test_store_tool_run_error() {
    let client = test_client().await;

    client
        .store_tool_run(
            "edit_file",
            &json!({"path": "/test/file.rs"}),
            &json!({"error": "File not found"}),
            50,
            true,
        )
        .await;
}

#[tokio::test]
async fn test_store_file_edit() {
    let client = test_client().await;

    client
        .store_file_edit(
            "src/main.rs",
            "Added new function for processing",
            Some("@@ -1,3 +1,5 @@\n+fn new_function() {\n+    // implementation\n+}\n"),
        )
        .await;
}

#[tokio::test]
async fn test_store_user_intent() {
    let client = test_client().await;

    client
        .store_user_intent(
            "Please add error handling to the login function",
            "Add error handling to login",
        )
        .await;
}

#[tokio::test]
async fn test_store_error() {
    let client = test_client().await;

    client
        .store_error(
            "tool_execution",
            "Command timed out after 30 seconds",
            Some("cargo test -- --nocapture"),
        )
        .await;
}

#[tokio::test]
async fn test_store_decision() {
    let client = test_client().await;

    client
        .store_decision(
            "Use SQLite for local caching",
            Some("SQLite provides good performance for local operations and doesn't require a separate server"),
        )
        .await;
}

#[tokio::test]
async fn test_store_project_convention() {
    let client = test_client().await;

    client
        .store_project_convention(
            "locuscodes",
            "Use snake_case for function names",
            vec![
                "fn process_data() {}",
                "fn calculate_total() {}",
            ],
        )
        .await;
}

#[tokio::test]
async fn test_store_skill() {
    let client = test_client().await;

    client
        .store_skill(
            "error_recovery",
            "Standard pattern for recovering from errors",
            vec![
                "Log the error with context",
                "Check if error is recoverable",
                "Retry with exponential backoff if appropriate",
                "Fall back to alternative approach if retry fails",
            ],
            true,
        )
        .await;
}
