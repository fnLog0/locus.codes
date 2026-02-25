//! Pre-built hooks for storing events at specific points in the agent loop.
//!
//! Each hook builds a `CreateEventRequest` and fires it async (non-blocking).
//! Hooks automatically set semantic links (related_to, extends, reinforces,
//! contradicts) based on event semantics, and merge with caller-provided links.
//!
//! ## Source Priority (high → low)
//!
//! | Source       | Confidence | Use Case                          |
//! |--------------|------------|-----------------------------------|
//! | `validator`  | 0.9        | Authoritative, runtime-verified   |
//! | `executor`   | 0.8        | Reliable, task completion         |
//! | `user`       | 0.7        | Valuable but subjective           |
//! | `agent`      | 0.6        | Agent decisions                   |
//! | `system`     | 0.5        | System events (default)           |
//!
//! ## Context IDs
//!
//! Backend requires format `type:name` (e.g. fact:redis_caching). We use the `fact` type throughout.
//!
//! **Constants** (for links; type aligned with event_kind):
//! | Constant              | Value                   | Use Case          |
//! | CONTEXT_USER_INTENT   | observation:user_intent | User messages     |
//! | CONTEXT_DECISIONS     | decision:decisions      | Agent reasoning   |
//! | CONTEXT_ERRORS        | observation:errors      | Error tracking    |
//! | CONTEXT_TOOLS         | fact:tools              | Tool schemas      |
//! | CONTEXT_SESSIONS      | fact:sessions           | Session/turn      |
//!
//! **Dynamic context_id** (type matches event_kind): action:*, observation:*, decision:*, fact:*

use crate::client::LocusGraphClient;
use crate::types::{CreateEventRequest, EventKind, TurnSummary};
use serde_json::json;

/// Context ID constants (one per feedback loop).
/// Backend requires format `type:name` (e.g. fact:redis_caching). Type is aligned with event_kind.
pub const CONTEXT_DECISIONS: &str = "decision:decisions";
pub const CONTEXT_ERRORS: &str = "observation:errors";
pub const CONTEXT_SESSIONS: &str = "fact:sessions";
pub const CONTEXT_TOOLS: &str = "fact:tools";
pub const CONTEXT_USER_INTENT: &str = "observation:user_intent";

impl LocusGraphClient {
    /// Create a session event at session start.
    ///
    /// context_id: `session:{slug}_{session_id}`
    /// extends: `{repo_hash}:sessions`
    pub async fn store_session_start(
        &self,
        session_slug: &str,
        session_id: &str,
        title: &str,
        repo_hash: &str,
    ) {
        let ctx = format!("session:{}_{}", safe_context_name(session_slug), session_id);
        let sessions_master = format!("{}:sessions", safe_context_name(repo_hash));

        let event = CreateEventRequest::new(
            EventKind::Fact,
            json!({
                "kind": "session_start",
                "data": {
                    "title": title,
                    "slug": session_slug,
                    "session_id": session_id,
                    "status": "active",
                    "turn_count": 0,
                    "totals": {
                        "events": 0,
                        "tool_calls": 0,
                        "llm_calls": 0,
                        "prompt_tokens": 0,
                        "completion_tokens": 0,
                        "files_modified": [],
                        "errors": 0,
                        "errors_resolved": 0
                    }
                }
            }),
        )
        .context_id(ctx)
        .extends(vec![sessions_master])
        .source("system");

        self.store_event(event).await;
    }

    /// Close a session with final stats.
    ///
    /// Same context_id as start — auto-overrides in LocusGraph.
    pub async fn store_session_end(
        &self,
        session_slug: &str,
        session_id: &str,
        summary: &str,
        turn_count: u32,
        totals: serde_json::Value,
    ) {
        let ctx = format!("session:{}_{}", safe_context_name(session_slug), session_id);

        let event = CreateEventRequest::new(
            EventKind::Fact,
            json!({
                "kind": "session_end",
                "data": {
                    "status": "closed",
                    "turn_count": turn_count,
                    "summary": summary,
                    "totals": totals,
                }
            }),
        )
        .context_id(ctx)
        .source("system");

        self.store_event(event).await;
    }

    /// Create a turn anchor at turn START.
    ///
    /// context_id: `turn:{session_id}_{turn_id}`
    /// extends: `session:{slug}_{session_id}`
    pub async fn store_turn_start(
        &self,
        session_id: &str,
        session_ctx: &str,
        turn_sequence: u32,
        user_message: &str,
    ) {
        let turn_id = format!("{:03}", turn_sequence);
        let ctx = format!("turn:{}_{}", session_id, turn_id);

        let event = CreateEventRequest::new(
            EventKind::Observation,
            json!({
                "kind": "turn_start",
                "data": {
                    "turn_id": turn_id,
                    "sequence": turn_sequence,
                    "status": "active",
                    "user_message": truncate_string(user_message, 1000),
                }
            }),
        )
        .context_id(ctx)
        .extends(vec![session_ctx.to_string()])
        .source("system");

        self.store_event(event).await;
    }

    /// Update turn anchor with summary at turn END.
    ///
    /// Same context_id as start — auto-overrides.
    pub async fn store_turn_end(
        &self,
        session_id: &str,
        session_ctx: &str,
        turn_sequence: u32,
        summary: TurnSummary,
    ) {
        let turn_id = format!("{:03}", turn_sequence);
        let ctx = format!("turn:{}_{}", session_id, turn_id);

        let event = CreateEventRequest::new(
            EventKind::Observation,
            json!({
                "kind": "turn_end",
                "data": {
                    "turn_id": turn_id,
                    "sequence": turn_sequence,
                    "status": "completed",
                    "title": summary.title,
                    "user_request": summary.user_request,
                    "actions_taken": summary.actions_taken,
                    "outcome": summary.outcome,
                    "decisions": summary.decisions,
                    "files_read": summary.files_read,
                    "files_modified": summary.files_modified,
                    "event_count": summary.event_count,
                }
            }),
        )
        .context_id(ctx)
        .extends(vec![session_ctx.to_string()])
        .source("agent");

        self.store_event(event).await;
    }

    /// Store any event during a turn (the full timeline).
    ///
    /// context_id: `{event_type}:{session_id}_{turn_id}_{seq}`
    /// extends: `turn:{session_id}_{turn_id}`
    #[allow(clippy::too_many_arguments)]
    pub async fn store_turn_event(
        &self,
        event_type: &str,
        session_id: &str,
        turn_id: &str,
        seq: u32,
        event_kind: EventKind,
        source: &str,
        payload: serde_json::Value,
        related_to: Option<Vec<String>>,
    ) {
        let ctx = format!(
            "{}:{}_{}_{:03}",
            safe_context_name(event_type),
            session_id,
            turn_id,
            seq
        );
        let turn_ctx = format!("turn:{}_{}", session_id, turn_id);

        let mut event = CreateEventRequest::new(event_kind, payload)
            .context_id(ctx)
            .extends(vec![turn_ctx])
            .source(source);

        if let Some(refs) = related_to {
            event = event.related_to(refs);
        }

        self.store_event(event).await;
    }

    /// Store a codebase snapshot at turn boundaries.
    ///
    /// context_id: `snapshot:{session_id}_{turn_id}_{seq}`
    #[allow(clippy::too_many_arguments)]
    pub async fn store_snapshot(
        &self,
        session_id: &str,
        turn_id: &str,
        seq: u32,
        git_head: &str,
        git_branch: &str,
        git_dirty: Vec<String>,
        git_staged: Vec<String>,
        snapshot_type: &str, // "turn_start" or "turn_end"
    ) {
        let ctx = format!("snapshot:{}_{}_{:03}", session_id, turn_id, seq);
        let turn_ctx = format!("turn:{}_{}", session_id, turn_id);

        let event = CreateEventRequest::new(
            EventKind::Fact,
            json!({
                "kind": "codebase_snapshot",
                "data": {
                    "git_head": git_head,
                    "git_branch": git_branch,
                    "git_dirty": git_dirty,
                    "git_staged": git_staged,
                    "snapshot_type": snapshot_type,
                    "seq": seq,
                }
            }),
        )
        .context_id(ctx)
        .extends(vec![turn_ctx])
        .source("system");

        self.store_event(event).await;
    }

    /// Bootstrap the sessions master event (cold start).
    ///
    /// context_id: `{repo_hash}:sessions`
    pub async fn bootstrap_sessions_master(
        &self,
        repo_hash: &str,
        project_anchor: &str,
    ) {
        let ctx = format!("{}:sessions", safe_context_name(repo_hash));

        let event = CreateEventRequest::new(
            EventKind::Fact,
            json!({
                "kind": "sessions_master",
                "data": {
                    "active_session": null,
                    "total_sessions": 0
                }
            }),
        )
        .context_id(ctx)
        .extends(vec![project_anchor.to_string()])
        .source("system");

        self.store_event(event).await;
    }
}

/// Truncate a string to a maximum length (UTF-8 safe).
fn truncate_string(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }
    // Find the last char boundary at or before max_len
    let mut end = max_len;
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    &s[..end]
}

/// Sanitize a string for use in context_id name part (backend expects type:name, name = [a-z0-9_]).
fn safe_context_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}


