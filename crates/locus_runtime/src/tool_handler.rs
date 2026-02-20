//! Tool execution handler for the Runtime.
//!
//! This module provides functions to execute tools via ToolBus,
//! emit events, and store results to memory.
//!
//! Meta-tools `tool_search`, `tool_explain`, and `task` are handled here
//! before reaching ToolBus (they query LocusGraph or spawn sub-runtimes).

use std::sync::Arc;
use std::time::Instant;

use locus_core::{ContentBlock, SessionEvent, ToolResultData, ToolUse, Turn};
use locus_graph::{ContextTypeFilter, LocusGraphClient, RetrieveOptions};
use locus_toolbus::ToolBus;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::error::RuntimeError;
use crate::memory;

/// Handle a single tool call.
///
/// Meta-tools `tool_search` and `tool_explain` are handled here (LocusGraph / cached schema).
/// The `task` tool is handled in the runtime (spawns sub-agent). All others go to ToolBus.
///
/// This function:
/// 1. Emits a ToolStart event
/// 2. Executes the tool (meta-tool or ToolBus)
/// 3. Stores the result to memory
/// 4. Emits a ToolDone event
/// 5. Returns the result for adding to session
pub async fn handle_tool_call(
    tool: ToolUse,
    toolbus: &Arc<ToolBus>,
    locus_graph: Arc<LocusGraphClient>,
    event_tx: &mpsc::Sender<SessionEvent>,
) -> Result<ToolResultData, RuntimeError> {
    info!("Executing tool: {} (id: {})", tool.name, tool.id);

    // Handle meta-tools directly (don't go through ToolBus)
    match tool.name.as_str() {
        "tool_search" => return handle_tool_search(&tool, Arc::clone(&locus_graph), event_tx).await,
        "tool_explain" => return handle_tool_explain(&tool, toolbus, event_tx).await,
        _ => {}
    }

    // Emit tool start event
    let _ = event_tx
        .send(SessionEvent::tool_start(tool.clone()))
        .await;

    // Execute via ToolBus
    let start = Instant::now();
    let result = toolbus.call(&tool.name, tool.args.clone()).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    let tool_result = match result {
        Ok((output, _duration_from_toolbus)) => {
            info!("Tool {} completed successfully in {}ms", tool.name, duration_ms);
            ToolResultData::success(output, duration_ms)
        }
        Err(e) => {
            error!("Tool {} failed: {}", tool.name, e);

            // Store error to memory (fire-and-forget)
            // Try to extract file path from tool.file_path, then fall back to args
            let file_path = tool.file_path.as_ref().and_then(|p| p.to_str())
                .or_else(|| tool.args.get("file_path").and_then(|p| p.as_str()))
                .or_else(|| tool.args.get("path").and_then(|p| p.as_str()))
                .or_else(|| tool.args.get("file").and_then(|p| p.as_str()));
            memory::store_error(
                Arc::clone(&locus_graph),
                format!("tool_{}", tool.name),
                e.to_string(),
                file_path.map(|s| s.to_string()),
            );

            ToolResultData::error(
                serde_json::json!({ "error": e.to_string() }),
                duration_ms,
            )
        }
    };

    // Store tool run to memory (fire-and-forget)
    memory::store_tool_run(
        Arc::clone(&locus_graph),
        tool.name.clone(),
        tool.args.clone(),
        tool_result.clone(),
    );

    // Store tool usage for discovery learning (fire-and-forget)
    if !tool_result.is_error {
        let graph = Arc::clone(&locus_graph);
        let tool_name = tool.name.clone();
        tokio::spawn(async move {
            graph.store_tool_usage(&tool_name, "", true, duration_ms).await;
        });
    }

    // Emit tool done event
    let _ = event_tx
        .send(SessionEvent::tool_done(tool.id.clone(), tool_result.clone()))
        .await;

    // If this was a file edit, store to memory
    if is_file_edit_tool(&tool.name) {
        if let Some(file_path) = &tool.file_path {
            let summary = format!("{} on {}", tool.name, file_path.display());
            memory::store_file_edit(
                locus_graph,
                file_path.to_string_lossy().to_string(),
                summary,
                None,
            );
        }
    }

    Ok(tool_result)
}

/// Handle tool_search meta-tool: query LocusGraph for tools matching the user's intent.
async fn handle_tool_search(
    tool: &ToolUse,
    locus_graph: Arc<LocusGraphClient>,
    event_tx: &mpsc::Sender<SessionEvent>,
) -> Result<ToolResultData, RuntimeError> {
    let start = Instant::now();

    let query = tool.args.get("query").and_then(|v| v.as_str()).unwrap_or("");
    let max_results = tool.args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(5);

    let _ = event_tx.send(SessionEvent::tool_start(tool.clone())).await;

    let options = RetrieveOptions::new()
        .limit(max_results)
        .context_type("fact", ContextTypeFilter::new().name("tool"));

    let result = locus_graph
        .retrieve_memories(query, Some(options))
        .await
        .unwrap_or_else(|_| locus_graph::ContextResult {
            memories: String::new(),
            items_found: 0,
            degraded: true,
        });

    let duration_ms = start.elapsed().as_millis() as u64;
    let output = serde_json::json!({
        "results": result.memories,
        "items_found": result.items_found,
    });

    let tool_result = ToolResultData::success(output, duration_ms);
    let _ = event_tx
        .send(SessionEvent::tool_done(tool.id.clone(), tool_result.clone()))
        .await;

    // Store run and usage for meta-tool
    memory::store_tool_run(
        Arc::clone(&locus_graph),
        tool.name.clone(),
        tool.args.clone(),
        tool_result.clone(),
    );
    if !tool_result.is_error {
        let graph = Arc::clone(&locus_graph);
        let tool_name = tool.name.clone();
        let intent = query.to_string();
        tokio::spawn(async move {
            graph.store_tool_usage(&tool_name, &intent, true, duration_ms).await;
        });
    }

    Ok(tool_result)
}

/// Handle tool_explain meta-tool: return full schema for a tool from ToolBus.
async fn handle_tool_explain(
    tool: &ToolUse,
    toolbus: &Arc<ToolBus>,
    event_tx: &mpsc::Sender<SessionEvent>,
) -> Result<ToolResultData, RuntimeError> {
    let start = Instant::now();

    let tool_id = tool.args.get("tool_id").and_then(|v| v.as_str()).unwrap_or("");

    let _ = event_tx.send(SessionEvent::tool_start(tool.clone())).await;

    let all_tools = toolbus.list_tools();
    let found = all_tools.iter().find(|t| t.name == tool_id);

    let duration_ms = start.elapsed().as_millis() as u64;
    let output = match found {
        Some(t) => serde_json::json!({
            "tool_id": t.name,
            "description": t.description,
            "parameters": t.parameters,
        }),
        None => serde_json::json!({
            "error": format!("Tool '{}' not found", tool_id),
        }),
    };

    let tool_result = ToolResultData::success(output, duration_ms);
    let _ = event_tx
        .send(SessionEvent::tool_done(tool.id.clone(), tool_result.clone()))
        .await;

    Ok(tool_result)
}

/// Handle multiple tool calls in sequence.
///
/// Executes tools one by one, collecting results.
pub async fn handle_tool_calls(
    tools: Vec<ToolUse>,
    toolbus: &Arc<ToolBus>,
    locus_graph: Arc<LocusGraphClient>,
    event_tx: &mpsc::Sender<SessionEvent>,
) -> Vec<(ToolUse, ToolResultData)> {
    let mut results = Vec::with_capacity(tools.len());

    for tool in tools {
        match handle_tool_call(tool.clone(), toolbus, Arc::clone(&locus_graph), event_tx).await {
            Ok(result) => results.push((tool, result)),
            Err(e) => {
                warn!("Tool call {} failed: {}", tool.id, e);
                let error_result = ToolResultData::error(
                    serde_json::json!({ "error": e.to_string() }),
                    0,
                );
                results.push((tool, error_result));
            }
        }
    }

    results
}

/// Create a tool result turn from tool execution results.
///
/// This creates a Turn with Tool role containing the results.
pub fn create_tool_result_turn(results: &[(ToolUse, ToolResultData)]) -> Turn {
    let mut turn = Turn::new(locus_core::Role::Tool);

    for (tool_use, result) in results {
        turn = turn.with_block(ContentBlock::tool_result(ToolResultData {
            output: serde_json::json!({
                "tool_use_id": tool_use.id,
                "tool_name": tool_use.name,
                "result": result.output,
                "duration_ms": result.duration_ms,
                "is_error": result.is_error,
            }),
            duration_ms: result.duration_ms,
            is_error: result.is_error,
        }));
    }

    turn
}

/// Check if a tool name is a file-editing tool.
fn is_file_edit_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "edit_file" | "create_file" | "undo_edit" | "write"
    )
}

/// Parse tool calls from LLM response content.
///
/// Extracts tool use blocks from an assistant turn.
pub fn extract_tool_calls(turn: &Turn) -> Vec<ToolUse> {
    turn.blocks
        .iter()
        .filter_map(|block| {
            if let ContentBlock::ToolUse { tool_use } = block {
                Some(tool_use.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Check if a tool call requires user confirmation.
///
/// Destructive operations should be confirmed before execution.
///
/// # Security Warning
/// This is a "seatbelt" check only - it provides basic protection against
/// obviously destructive commands but is NOT a security boundary. The patterns
/// can be bypassed via:
/// - Extra whitespace: `rm  -rf` (multiple spaces)
/// - Pipes: `echo "rm -rf /" | sh`
/// - Shell aliases or variable expansion
/// - Backslash escaping: `\rm`
///
/// For real security, implement sandboxing (Docker, user namespace, or seccomp).
/// Consider using a proper shell parser instead of substring matching for
/// production use.
pub fn requires_confirmation(tool: &ToolUse) -> bool {
    match tool.name.as_str() {
        "bash" => {
            // Check for potentially destructive commands
            if let Some(cmd) = tool.args.get("command").and_then(|c| c.as_str()) {
                let dangerous_patterns = [
                    "rm ",
                    "rm -",
                    "rmdir",
                    "git push",
                    "git reset --hard",
                    "DROP",
                    "TRUNCATE",
                    "DELETE FROM",
                    ":(){ :|:& };:",
                    "mkfs",
                    "dd if=",
                    "> /dev/",
                    "chmod -R 777",
                    "chown -R",
                ];

                let cmd_lower = cmd.to_lowercase();
                for pattern in dangerous_patterns {
                    if cmd_lower.contains(&pattern.to_lowercase()) {
                        return true;
                    }
                }
            }
            false
        }
        "edit_file" | "create_file" => {
            // File operations on sensitive paths should be confirmed
            if let Some(path) = tool.file_path.as_ref() {
                let path_str = path.to_string_lossy().to_lowercase();
                let sensitive_paths = [
                    ".env",
                    ".ssh",
                    ".gnupg",
                    "credentials",
                    "secrets",
                    "id_rsa",
                    "authorized_keys",
                ];

                for sensitive in sensitive_paths {
                    if path_str.contains(sensitive) {
                        return true;
                    }
                }
            }
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_file_edit_tool() {
        assert!(is_file_edit_tool("edit_file"));
        assert!(is_file_edit_tool("create_file"));
        assert!(is_file_edit_tool("undo_edit"));
        assert!(is_file_edit_tool("write"));
        assert!(!is_file_edit_tool("bash"));
        assert!(!is_file_edit_tool("read"));
    }

    #[test]
    fn test_requires_confirmation_bash_rm() {
        let tool = ToolUse::new(
            "t1",
            "bash",
            serde_json::json!({ "command": "rm -rf /important" }),
        );

        assert!(requires_confirmation(&tool));
    }

    #[test]
    fn test_requires_confirmation_bash_safe() {
        let tool = ToolUse::new(
            "t1",
            "bash",
            serde_json::json!({ "command": "ls -la" }),
        );

        assert!(!requires_confirmation(&tool));
    }

    #[test]
    fn test_requires_confirmation_env_file() {
        let tool = ToolUse::new("t1", "edit_file", serde_json::json!({}))
            .with_file_path(PathBuf::from("/project/.env"));

        assert!(requires_confirmation(&tool));
    }

    #[test]
    fn test_requires_confirmation_normal_file() {
        let tool = ToolUse::new("t1", "edit_file", serde_json::json!({}))
            .with_file_path(PathBuf::from("/project/src/main.rs"));

        assert!(!requires_confirmation(&tool));
    }

    #[test]
    fn test_extract_tool_calls() {
        let tool = ToolUse::new("t1", "bash", serde_json::json!({"command": "ls"}));
        let turn = Turn::assistant()
            .with_block(ContentBlock::text("Let me check the files"))
            .with_block(ContentBlock::tool_use(tool.clone()));

        let calls = extract_tool_calls(&turn);

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "bash");
    }

    #[test]
    fn test_extract_tool_calls_empty() {
        let turn = Turn::assistant().with_block(ContentBlock::text("No tools here"));

        let calls = extract_tool_calls(&turn);

        assert!(calls.is_empty());
    }

    #[test]
    fn test_create_tool_result_turn() {
        let tool = ToolUse::new("t1", "bash", serde_json::json!({}));
        let result = ToolResultData::success(serde_json::json!({"output": "hello"}), 50);

        let results = vec![(tool, result)];
        let turn = create_tool_result_turn(&results);

        assert_eq!(turn.role, locus_core::Role::Tool);
        assert_eq!(turn.blocks.len(), 1);
    }
}
