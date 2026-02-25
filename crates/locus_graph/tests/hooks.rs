//! Tests for LocusGraph client hooks (session/turn API).
//!
//! These tests require a running LocusGraph gRPC server at localhost:50051.

mod common;

use common::test_client;
use locus_graph::{EventKind, TurnSummary};

#[tokio::test]
async fn test_store_turn_event() {
    let client = test_client().await;

    client
        .store_turn_event(
            "decision",
            "session-123",
            "001",
            1,
            EventKind::Decision,
            "agent",
            serde_json::json!({
                "kind": "decision",
                "data": { "summary": "Use SQLite for cache", "reasoning": null }
            }),
            Some(vec!["decision:decisions".to_string()]),
        )
        .await;
}

#[tokio::test]
async fn test_store_session_start_and_end() {
    let client = test_client().await;

    client
        .store_session_start(
            "test-session",
            "sess-abc",
            "Test session title",
            "repohash123",
        )
        .await;

    client
        .store_session_end(
            "test-session",
            "sess-abc",
            "Session completed",
            3,
            serde_json::json!({
                "events": 10,
                "tool_calls": 5,
                "llm_calls": 2,
                "prompt_tokens": 1000,
                "completion_tokens": 500,
            }),
        )
        .await;
}

#[tokio::test]
async fn test_store_turn_start_and_end() {
    let client = test_client().await;
    let session_id = "sess-abc123";
    let session_ctx = "session:test-session_sess-abc123";

    client
        .store_turn_start(session_id, session_ctx, 1, "Please add tests")
        .await;

    let summary = TurnSummary {
        title: "Add tests".to_string(),
        user_request: "Please add tests".to_string(),
        actions_taken: vec!["Created test module".to_string()],
        outcome: "Tests added".to_string(),
        decisions: vec![],
        files_read: vec!["src/lib.rs".to_string()],
        files_modified: vec!["tests/foo.rs".to_string()],
        event_count: 5,
    };

    client
        .store_turn_end(session_id, session_ctx, 1, summary)
        .await;
}
