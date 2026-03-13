use locus_graph::{CreateEventRequest, EventKind, LocusGraphClient};

use super::safe_context_name;

/// Build the project root anchor context_id.
/// Format: "project:{project_name}_{repo_hash}"
pub fn project_anchor_id(project_name: &str, repo_hash: &str) -> String {
    format!(
        "project:{}_{}",
        safe_context_name(project_name),
        safe_context_name(repo_hash)
    )
}

/// Build the tool anchor context_id.
///
/// Format: "tool_anchor:{project_name}_{repo_hash}"
pub fn tool_anchor_id(project_name: &str, repo_hash: &str) -> String {
    format!(
        "tool_anchor:{}_{}",
        safe_context_name(project_name),
        safe_context_name(repo_hash)
    )
}

/// Build the session anchor context_id.
///
/// Format: "session_anchor:{project_name}_{repo_hash}"
pub fn session_anchor_id(project_name: &str, repo_hash: &str) -> String {
    format!(
        "session_anchor:{}_{}",
        safe_context_name(project_name),
        safe_context_name(repo_hash)
    )
}

/// Build a session context_id.
///
/// Format: "session:{slug}_{session_id_short}"
/// session_id_short is first 8 chars of the UUID.
pub fn session_context_id(slug: &str, session_id: &str) -> String {
    let short_id = if session_id.len() > 8 {
        &session_id[..8]
    } else {
        session_id
    };
    format!(
        "session:{}_{}",
        safe_context_name(slug),
        safe_context_name(short_id)
    )
}

/// Ensure the session anchor exists in LocusGraph.
/// Idempotent — same context_id = overwrite.
/// Called at Runtime::new() after ensure_project_anchor.
pub async fn ensure_session_anchor(
    locus_graph: &LocusGraphClient,
    project_name: &str,
    repo_hash: &str,
) {
    let anchor_id = session_anchor_id(project_name, repo_hash);
    let project_anchor = project_anchor_id(project_name, repo_hash);

    let event = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "session_anchor",
            "data": {
                "project_name": project_name,
                "repo_hash": repo_hash,
                "active_session": null,
            }
        }),
    )
    .context_id(anchor_id)
    .extends(vec![project_anchor])
    .source("validator");

    locus_graph.store_event(event).await;
}

/// Ensure the project root anchor exists in LocusGraph.
/// Idempotent — same context_id = overwrite.
/// Called at Runtime::new() before anything else.
pub async fn ensure_project_anchor(
    locus_graph: &LocusGraphClient,
    project_name: &str,
    repo_hash: &str,
    repo_root: &std::path::Path,
) {
    let anchor_id = project_anchor_id(project_name, repo_hash);

    let event = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "project_anchor",
            "data": {
                "project_name": project_name,
                "repo_hash": repo_hash,
                "repo_root": repo_root.to_string_lossy(),
                "created_at": chrono::Utc::now().to_rfc3339(),
            }
        }),
    )
    .context_id(anchor_id)
    .source("validator");

    locus_graph.store_event(event).await;
}
