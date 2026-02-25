//! Memory recall and storage helpers for the Runtime.
//!
//! These functions wrap LocusGraph operations for use in the agent loop.
//! All storage operations are fire-and-forget (non-blocking).

use std::sync::Arc;

use locus_core::{SessionEvent, SessionId, ToolResultData};
use locus_graph::{
    ContextResult, CreateEventRequest, EventKind, LocusGraphClient, RetrieveOptions,
    CONTEXT_DECISIONS, CONTEXT_ERRORS, CONTEXT_TOOLS, CONTEXT_USER_INTENT,
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
/// Combines sessions master, session, and global context IDs for relevant memory retrieval.
pub fn build_context_ids(repo_hash: &str, session_id: &SessionId) -> Vec<String> {
    vec![
        format!("{}:sessions", repo_hash),
        format!("session:{}", session_id.as_str()),
        CONTEXT_DECISIONS.to_string(),
        CONTEXT_ERRORS.to_string(),
        CONTEXT_USER_INTENT.to_string(),
        CONTEXT_TOOLS.to_string(),
    ]
}

/// Store turn-scoped decision (fire-and-forget).
pub fn store_turn_decision(
    locus_graph: Arc<LocusGraphClient>,
    session_id: String,
    turn_id: String,
    seq: u32,
    summary: String,
    reasoning: Option<String>,
) {
    tokio::spawn(async move {
        locus_graph
            .store_turn_event(
                "decision",
                &session_id,
                &turn_id,
                seq,
                EventKind::Decision,
                "agent",
                serde_json::json!({
                    "kind": "decision",
                    "data": {
                        "summary": summary,
                        "reasoning": reasoning,
                    }
                }),
                Some(vec!["decision:decisions".to_string()]),
            )
            .await;
    });
}

/// Store turn-scoped tool run (fire-and-forget).
pub fn store_turn_tool_run(
    locus_graph: Arc<LocusGraphClient>,
    session_id: String,
    turn_id: String,
    seq: u32,
    tool_name: String,
    args: serde_json::Value,
    result: ToolResultData,
) {
    tokio::spawn(async move {
        locus_graph
            .store_turn_event(
                "action",
                &session_id,
                &turn_id,
                seq,
                if result.is_error {
                    EventKind::Observation
                } else {
                    EventKind::Action
                },
                "executor",
                serde_json::json!({
                    "kind": "tool_run",
                    "data": {
                        "tool": tool_name,
                        "args": args,
                        "result_preview": result.output,
                        "duration_ms": result.duration_ms,
                        "is_error": result.is_error,
                    }
                }),
                None,
            )
            .await;
    });
}

/// Store turn-scoped error (fire-and-forget).
pub fn store_turn_error(
    locus_graph: Arc<LocusGraphClient>,
    session_id: String,
    turn_id: String,
    seq: u32,
    context: String,
    error_message: String,
) {
    tokio::spawn(async move {
        locus_graph
            .store_turn_event(
                "error",
                &session_id,
                &turn_id,
                seq,
                EventKind::Observation,
                "system",
                serde_json::json!({
                    "kind": "error",
                    "data": {
                        "context": context,
                        "error_message": error_message,
                    }
                }),
                Some(vec!["observation:errors".to_string()]),
            )
            .await;
    });
}

/// Store turn-scoped file edit (fire-and-forget).
pub fn store_turn_file_edit(
    locus_graph: Arc<LocusGraphClient>,
    session_id: String,
    turn_id: String,
    seq: u32,
    path: String,
    summary: String,
) {
    tokio::spawn(async move {
        locus_graph
            .store_turn_event(
                "file",
                &session_id,
                &turn_id,
                seq,
                EventKind::Action,
                "executor",
                serde_json::json!({
                    "kind": "file_edit",
                    "data": {
                        "path": path,
                        "summary": summary,
                    }
                }),
                None,
            )
            .await;
    });
}

/// Store turn-scoped LLM call (fire-and-forget).
#[allow(clippy::too_many_arguments)]
pub fn store_turn_llm_call(
    locus_graph: Arc<LocusGraphClient>,
    session_id: String,
    turn_id: String,
    seq: u32,
    model: String,
    prompt_tokens: u64,
    completion_tokens: u64,
    duration_ms: u64,
) {
    tokio::spawn(async move {
        locus_graph
            .store_turn_event(
                "llm",
                &session_id,
                &turn_id,
                seq,
                EventKind::Fact,
                "system",
                serde_json::json!({
                    "kind": "llm_call",
                    "data": {
                        "model": model,
                        "prompt_tokens": prompt_tokens,
                        "completion_tokens": completion_tokens,
                        "duration_ms": duration_ms,
                    }
                }),
                None,
            )
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

/// Bootstrap tool knowledge in LocusGraph (cold start).
///
/// Implements Steps 2-4 from tools.md:
/// - Step 2: Create tool registry master event (`{repo_hash}:tools`)
/// - Step 3: Create individual tool events (`tools:{tool_name}`)
/// - Step 4: Create meta-tool events (`meta:{tool_name}`)
///
/// All events extend the project anchor and are idempotent (same context_id = override).
pub fn bootstrap_tools(
    locus_graph: Arc<LocusGraphClient>,
    repo_hash: String,
    project_anchor: String,
    tools: Vec<ToolInfo>,
    meta_tools: Vec<ToolInfo>,
    locus_version: String,
) {
    tokio::spawn(async move {
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        let meta_names: Vec<&str> = meta_tools.iter().map(|t| t.name.as_str()).collect();

        // Step 2: Tool registry master event
        let tools_master_ctx = format!("{}:tools", safe_context_name(&repo_hash));
        let master_event = CreateEventRequest::new(
            EventKind::Fact,
            serde_json::json!({
                "kind": "tools_master",
                "data": {
                    "tool_count": tools.len() + meta_tools.len(),
                    "tool_names": tool_names,
                    "meta_names": meta_names,
                    "locus_version": locus_version,
                }
            }),
        )
        .context_id(tools_master_ctx.clone())
        .extends(vec![project_anchor.clone()])
        .source("validator");

        locus_graph.store_event(master_event).await;

        // Step 3: Individual tool events
        for tool in &tools {
            let tool_ctx = format!("tools:{}", safe_context_name(&tool.name));
            let tool_event = CreateEventRequest::new(
                EventKind::Fact,
                serde_json::json!({
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                }),
            )
            .context_id(tool_ctx)
            .extends(vec![tools_master_ctx.clone()])
            .related_to(vec![project_anchor.clone()])
            .source("validator");

            locus_graph.store_event(tool_event).await;
        }

        // Step 4: Meta-tool events
        for tool in &meta_tools {
            let meta_ctx = format!("meta:{}", safe_context_name(&tool.name));
            let meta_event = CreateEventRequest::new(
                EventKind::Fact,
                serde_json::json!({
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                }),
            )
            .context_id(meta_ctx)
            .extends(vec![tools_master_ctx.clone()])
            .related_to(vec![project_anchor.clone()])
            .source("validator");

            locus_graph.store_event(meta_event).await;
        }
    });
}

/// Sanitize a string for use in context_id name part (backend expects type:name, name = [a-z0-9_-]).
fn safe_context_name(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_context_ids() {
        let session_id = SessionId::new();
        let ids = build_context_ids("abc123", &session_id);

        assert!(ids.contains(&"abc123:sessions".to_string()));
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
