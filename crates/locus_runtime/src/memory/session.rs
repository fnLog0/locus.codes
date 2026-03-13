use locus_graph::{CreateEventRequest, EventKind, LocusGraphClient};

use super::{project_anchor_id, session_anchor_id, session_context_id};

/// Store a new session event in LocusGraph and mark it active.
/// Called when the first user message arrives (session_slug is known).
pub async fn store_session_start(
    locus_graph: &LocusGraphClient,
    project_name: &str,
    repo_hash: &str,
    session_slug: &str,
    session_id: &str,
) {
    let session_ctx = session_context_id(session_slug, session_id);
    let anchor = session_anchor_id(project_name, repo_hash);

    // Create session event
    let session_event = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "session",
            "data": {
                "slug": session_slug,
                "session_id": session_id,
                "started_at": chrono::Utc::now().to_rfc3339(),
                "status": "active",
                "turn_count": 0,
            }
        }),
    )
    .context_id(session_ctx.clone())
    .extends(vec![anchor.clone()])
    .source("validator");

    locus_graph.store_event(session_event).await;

    // Update session_anchor with active_session
    let project_anchor = project_anchor_id(project_name, repo_hash);
    let anchor_update = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "session_anchor",
            "data": {
                "project_name": project_name,
                "repo_hash": repo_hash,
                "active_session": session_ctx,
            }
        }),
    )
    .context_id(anchor)
    .extends(vec![project_anchor])
    .source("validator");

    locus_graph.store_event(anchor_update).await;
}

/// Close a session in LocusGraph — update status, clear active_session.
/// Called at session shutdown.
pub async fn store_session_end(
    locus_graph: &LocusGraphClient,
    project_name: &str,
    repo_hash: &str,
    session_slug: &str,
    session_id: &str,
    turn_count: u32,
) {
    let session_ctx = session_context_id(session_slug, session_id);
    let anchor = session_anchor_id(project_name, repo_hash);

    // Update session with closed status (same context_id = overwrite)
    let session_event = CreateEventRequest::new(
        EventKind::Fact,
        serde_json::json!({
            "kind": "session",
            "data": {
                "slug": session_slug,
                "session_id": session_id,
                "status": "closed",
                "ended_at": chrono::Utc::now().to_rfc3339(),
                "turn_count": turn_count,
            }
        }),
    )
    .context_id(session_ctx)
    .extends(vec![anchor.clone()])
    .source("validator");

    locus_graph.store_event(session_event).await;

    // Clear active_session on session_anchor
    let project_anchor = project_anchor_id(project_name, repo_hash);
    let anchor_update = CreateEventRequest::new(
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
    .context_id(anchor)
    .extends(vec![project_anchor])
    .source("validator");

    locus_graph.store_event(anchor_update).await;
}
