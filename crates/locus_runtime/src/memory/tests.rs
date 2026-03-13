use super::turns;
use super::*;
use serde_json::json;

#[test]
fn test_build_context_ids() {
    let turn_contexts: Vec<String> = vec!["turn:test-session_turn-1".to_string()];
    let ids = build_context_ids(
        "locuscodes",
        "abc123",
        "test-session",
        "a1b2c3d4-uuid",
        &turn_contexts,
    );

    assert!(ids.contains(&"session_anchor:locuscodes_abc123".to_string()));
    assert!(ids.contains(&"tool_anchor:locuscodes_abc123".to_string()));
    assert!(ids.contains(&"session:test-session_a1b2c3d4".to_string()));
    assert!(ids.contains(&"turn:test-session_turn-1".to_string()));
}

#[test]
fn test_build_context_ids_empty_session() {
    let ids = build_context_ids("locuscodes", "abc123", "", "", &[]);

    assert!(ids.contains(&"session_anchor:locuscodes_abc123".to_string()));
    assert!(ids.contains(&"tool_anchor:locuscodes_abc123".to_string()));
    assert_eq!(ids.len(), 2);
}

#[test]
fn test_project_anchor_id() {
    let id = project_anchor_id("locuscodes", "abc123");
    assert_eq!(id, "project:locuscodes_abc123");
}

#[test]
fn test_project_anchor_id_sanitized() {
    let id = project_anchor_id("My Project!", "abc/123");
    assert_eq!(id, "project:my_project__abc_123");
}

#[test]
fn test_tool_anchor_id() {
    let id = tool_anchor_id("locuscodes", "abc123");
    assert_eq!(id, "tool_anchor:locuscodes_abc123");
}

#[test]
fn test_session_anchor_id() {
    let id = session_anchor_id("locuscodes", "abc123");
    assert_eq!(id, "session_anchor:locuscodes_abc123");
}

#[test]
fn test_session_context_id() {
    let id = session_context_id("fix-jwt-bug", "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
    assert_eq!(id, "session:fix-jwt-bug_a1b2c3d4");
}

#[test]
fn test_session_context_id_short() {
    let id = session_context_id("my-session", "abc123");
    assert_eq!(id, "session:my-session_abc123");
}

#[test]
fn test_simple_hash_consistency() {
    let hash1 = simple_hash("/path/to/repo");
    let hash2 = simple_hash("/path/to/repo");
    assert_eq!(hash1, hash2);
}

#[test]
fn test_simple_hash_different() {
    let hash1 = simple_hash("/path/to/repo1");
    let hash2 = simple_hash("/path/to/repo2");
    assert_ne!(hash1, hash2);
}

#[test]
fn test_build_turn_start() {
    let event = turns::build_turn_start(
        "turn:fix-jwt_validate-token",
        "session:fix-jwt_a1b2c3d4",
        "validate the JWT token",
        1,
    );
    assert_eq!(
        event.context_id.as_deref(),
        Some("turn:fix-jwt_validate-token")
    );
    assert!(event
        .extends
        .as_ref()
        .unwrap()
        .contains(&"session:fix-jwt_a1b2c3d4".to_string()));
}

#[test]
fn test_build_action_event() {
    let event = turns::build_action_event(
        "action:a1b2c3d4_001_001",
        "turn:fix-jwt_validate-token",
        "bash",
        &json!({"command": "cargo test"}),
        &json!({"output": "ok"}),
        false,
        150,
    );
    assert_eq!(event.context_id.as_deref(), Some("action:a1b2c3d4_001_001"));
    assert!(event
        .extends
        .as_ref()
        .unwrap()
        .contains(&"turn:fix-jwt_validate-token".to_string()));
}

#[test]
fn test_build_llm_event() {
    let event = turns::build_llm_event(
        "llm:a1b2c3d4_001_002",
        "turn:fix-jwt_validate-token",
        "claude-sonnet-4",
        1000,
        200,
        3500,
        true,
    );
    assert_eq!(event.context_id.as_deref(), Some("llm:a1b2c3d4_001_002"));
    let data = event.payload.get("data").unwrap();
    assert_eq!(data.get("total_tokens").unwrap(), 1200);
}
