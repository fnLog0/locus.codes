//! Memory recall and storage helpers for the Runtime.
//!
//! These functions wrap LocusGraph operations for use in the agent loop.
//! All storage operations are fire-and-forget (non-blocking).

use std::sync::Arc;

use locus_core::{SessionEvent, SessionId, ToolResultData};
use locus_graph::{
    ContextResult, EventLinks, LocusGraphClient, RetrieveOptions, CONTEXT_DECISIONS, CONTEXT_ERRORS,
    CONTEXT_TOOLS, CONTEXT_USER_INTENT,
};
use locus_toolbus::ToolInfo;
use tokio::sync::mpsc;
use tracing::warn;

/// Recall relevant memories before LLM call.
///
/// Queries LocusGraph for memories relevant to the query and emits
/// a MemoryRecall event to notify the TUI.
pub async fn recall_memories(
    locus_graph: &LocusGraphClient,
    event_tx: &mpsc::Sender<SessionEvent>,
    query: &str,
    memory_limit: u8,
    context_ids: Vec<String>,
) -> ContextResult {
    let mut options = RetrieveOptions::new().limit(memory_limit as u64);
    for id in context_ids {
        options = options.context_id(id);
    }

    let result = locus_graph
        .retrieve_memories(query, Some(options))
        .await
        .unwrap_or_else(|e| {
            warn!("Memory recall failed: {}", e);
            ContextResult {
                memories: String::new(),
                items_found: 0,
                degraded: true,
            }
        });

    if result.degraded {
        warn!("Memory service degraded - operating without memory context");
    }

    // Notify TUI about memory recall
    let _ = event_tx
        .send(SessionEvent::memory_recall(query, result.items_found))
        .await;

    result
}

/// Tools always available in every LLM call.
/// These are cheap, universally useful, and don't need discovery.
pub const CORE_TOOLS: &[&str] = &[
    "bash",
    "edit_file",
    "create_file",
    "undo_edit",
    "glob",
    "grep",
    "finder",
    "tool_search",
    "tool_explain",
];

/// Get the active tool list for an LLM call.
///
/// Returns core tools (always available) filtered from the full tool list.
/// In the future, this will also include LocusGraph-promoted hot tools.
pub fn get_active_tools(all_tools: &[ToolInfo]) -> Vec<ToolInfo> {
    all_tools
        .iter()
        .filter(|t| CORE_TOOLS.contains(&t.name.as_str()))
        .cloned()
        .collect()
}

/// Build context_ids for memory queries.
///
/// Combines project, session, and global context IDs for relevant memory retrieval.
pub fn build_context_ids(repo_hash: &str, session_id: &SessionId) -> Vec<String> {
    vec![
        format!("project:{}", repo_hash),
        CONTEXT_DECISIONS.to_string(),
        CONTEXT_ERRORS.to_string(),
        CONTEXT_USER_INTENT.to_string(),
        CONTEXT_TOOLS.to_string(),
        format!("session:{}", session_id.as_str()),
    ]
}

/// Store user intent (fire-and-forget).
///
/// Called when the user sends a message to track their intent.
/// Uses tokio::spawn to ensure the call is truly non-blocking.
pub fn store_user_intent(locus_graph: Arc<LocusGraphClient>, message: String, intent_summary: String) {
    tokio::spawn(async move {
        locus_graph.store_user_intent(&message, &intent_summary, EventLinks::default()).await;
    });
}

/// Store AI decision after a turn (fire-and-forget).
///
/// Called after the LLM responds to capture reasoning.
/// Uses tokio::spawn to ensure the call is truly non-blocking.
pub fn store_decision(locus_graph: Arc<LocusGraphClient>, summary: String, reasoning: Option<String>) {
    tokio::spawn(async move {
        locus_graph.store_decision(&summary, reasoning.as_deref(), EventLinks::default()).await;
    });
}

/// Store tool run result (fire-and-forget).
///
/// Called after executing a tool via ToolBus.
/// Uses tokio::spawn to ensure the call is truly non-blocking.
pub fn store_tool_run(
    locus_graph: Arc<LocusGraphClient>,
    tool_name: String,
    args: serde_json::Value,
    result: ToolResultData,
    links: EventLinks,
) {
    tokio::spawn(async move {
        locus_graph
            .store_tool_run(
                &tool_name,
                &args,
                &result.output,
                result.duration_ms,
                result.is_error,
                links,
            )
            .await;
    });
}

/// Store error (fire-and-forget).
///
/// Called when any error occurs in the agent loop.
/// Uses tokio::spawn to ensure the call is truly non-blocking.
pub fn store_error(
    locus_graph: Arc<LocusGraphClient>,
    context: String,
    error_message: String,
    command_or_file: Option<String>,
    links: EventLinks,
) {
    tokio::spawn(async move {
        locus_graph
            .store_error(&context, &error_message, command_or_file.as_deref(), links)
            .await;
    });
}

/// Store file edit (fire-and-forget).
///
/// Called after editing or creating a file.
/// Uses tokio::spawn to ensure the call is truly non-blocking.
pub fn store_file_edit(
    locus_graph: Arc<LocusGraphClient>,
    path: String,
    summary: String,
    diff_preview: Option<String>,
    links: EventLinks,
) {
    tokio::spawn(async move {
        locus_graph.store_file_edit(&path, &summary, diff_preview.as_deref(), links).await;
    });
}

/// Store LLM call (fire-and-forget).
///
/// Called after an LLM API call to track usage.
/// Uses tokio::spawn to ensure the call is truly non-blocking.
pub fn store_llm_call(
    locus_graph: Arc<LocusGraphClient>,
    model: String,
    prompt_tokens: u64,
    completion_tokens: u64,
    duration_ms: u64,
    is_error: bool,
) {
    tokio::spawn(async move {
        locus_graph
            .store_llm_call(&model, prompt_tokens, completion_tokens, duration_ms, is_error, EventLinks::default())
            .await;
    });
}

/// Simple hash function for repo paths.
pub fn simple_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_context_ids() {
        let session_id = SessionId::new();
        let ids = build_context_ids("abc123", &session_id);

        assert!(ids.contains(&"project:abc123".to_string()));
        assert!(ids.contains(&"decision:decisions".to_string()));
        assert!(ids.contains(&"observation:errors".to_string()));
        assert!(ids.contains(&"observation:user_intent".to_string()));
        assert!(ids.contains(&"fact:tools".to_string()));
        assert!(ids.iter().any(|id| id.starts_with("session:")));
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
}
