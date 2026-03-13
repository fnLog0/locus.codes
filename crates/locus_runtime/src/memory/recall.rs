use locus_core::SessionEvent;
use locus_graph::{ContextResult, LocusGraphClient, RetrieveOptions};
use locus_toolbus::ToolInfo;
use tokio::sync::mpsc;
use tracing::warn;

use super::{session_anchor_id, session_context_id, tool_anchor_id};

/// Recall relevant memories before LLM call.
///
/// Queries LocusGraph for memories relevant to the query and emits
/// a MemoryRecall event to notify the TUI.
pub async fn recall_memories(
    locus_graph: &LocusGraphClient,
    event_tx: &mpsc::Sender<SessionEvent>,
    query: &str,
    memory_limit: u8,
    context_ids: &[String],
) -> ContextResult {
    let mut options = RetrieveOptions::new().limit(memory_limit as u64);
    for id in context_ids {
        options = options.context_id(id.clone());
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
/// Combines sessions master, session, global context IDs, and known turn contexts.
pub fn build_context_ids(
    project_name: &str,
    repo_hash: &str,
    session_slug: &str,
    session_id: &str,
    turn_contexts: &[String],
) -> Vec<String> {
    let mut ids = vec![
        session_anchor_id(project_name, repo_hash),
        tool_anchor_id(project_name, repo_hash),
    ];

    if !session_slug.is_empty() && !session_id.is_empty() {
        ids.push(session_context_id(session_slug, session_id));
    }

    // Add all known turn contexts for this session
    for turn_ctx in turn_contexts {
        ids.push(turn_ctx.clone());
    }

    ids
}

/// Fetch existing turn contexts for a session from LocusGraph.
///
/// Call this at session start to get all previous turns for context retrieval.
pub async fn fetch_session_turns(
    locus_graph: &LocusGraphClient,
    session_slug: &str,
    _session_id: &str,
) -> Vec<String> {
    locus_graph
        .fetch_session_turns(session_slug)
        .await
        .unwrap_or_else(|e| {
            warn!("Failed to fetch session turns: {}", e);
            vec![]
        })
}
