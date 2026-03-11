//! Memory recall and storage helpers for the Runtime.
//!
//! These functions wrap LocusGraph operations for use in the agent loop.
//! All storage operations are fire-and-forget (non-blocking).

use locus_core::SessionEvent;
use locus_graph::{ContextResult, LocusGraphClient, RetrieveOptions, CONTEXT_TOOLS};
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
/// Combines sessions master, session, global context IDs, and known turn contexts.
pub fn build_context_ids(
    _project_name: &str,
    repo_hash: &str,
    _session_slug: &str,
    turn_contexts: &[String],
) -> Vec<String> {
    let mut ids = vec![format!("{}:sessions", repo_hash), CONTEXT_TOOLS.to_string()];

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
) -> Vec<String> {
    locus_graph
        .fetch_session_turns(session_slug)
        .await
        .unwrap_or_else(|e| {
            warn!("Failed to fetch session turns: {}", e);
            vec![]
        })
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
    locus_graph: std::sync::Arc<LocusGraphClient>,
    repo_hash: String,
    project_anchor: String,
    tools: Vec<ToolInfo>,
    meta_tools: Vec<ToolInfo>,
    locus_version: String,
) {
    use locus_graph::{CreateEventRequest, EventKind};

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

/// Build the project root anchor context_id.
/// Format: "project:{project_name}_{repo_hash}"
pub fn project_anchor_id(project_name: &str, repo_hash: &str) -> String {
    format!(
        "project:{}_{}",
        safe_context_name(project_name),
        safe_context_name(repo_hash)
    )
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
    use locus_graph::{CreateEventRequest, EventKind};

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
        let turn_contexts: Vec<String> = vec!["turn:test-session_turn-1".to_string()];
        let ids = build_context_ids("locuscodes", "abc123", "test-session", &turn_contexts);

        assert!(ids.contains(&"abc123:sessions".to_string())); // Phase 3 will change this
        assert!(ids.contains(&"fact:tools".to_string()));
        assert!(ids.contains(&"turn:test-session_turn-1".to_string()));
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
