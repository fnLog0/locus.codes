use std::sync::Arc;

use locus_graph::{CreateEventRequest, EventKind, LocusGraphClient};
use locus_toolbus::ToolInfo;

use super::{safe_context_name, tool_anchor_id};

/// Bootstrap tool knowledge in LocusGraph (cold start).
///
/// Implements Steps 2-4 from tools.md:
/// - Step 2: Create tool anchor event (`tool_anchor:{project_name}_{repo_hash}`)
/// - Step 3: Create individual tool events (`tool:{tool_name}`)
/// - Step 4: Create meta-tool events (`meta:{tool_name}`)
///
/// All events extend the project anchor and are idempotent (same context_id = override).
pub fn bootstrap_tools(
    locus_graph: Arc<LocusGraphClient>,
    project_name: String,
    repo_hash: String,
    project_anchor: String,
    tools: Vec<ToolInfo>,
    meta_tools: Vec<ToolInfo>,
    locus_version: String,
) {
    tokio::spawn(async move {
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        let meta_names: Vec<&str> = meta_tools.iter().map(|t| t.name.as_str()).collect();

        // Step 2: Tool anchor event
        let tool_anchor = tool_anchor_id(&project_name, &repo_hash);
        let master_event = CreateEventRequest::new(
            EventKind::Fact,
            serde_json::json!({
                "kind": "tool_anchor",
                "data": {
                    "tool_count": tools.len() + meta_tools.len(),
                    "tool_names": tool_names,
                    "meta_names": meta_names,
                    "locus_version": locus_version,
                }
            }),
        )
        .context_id(tool_anchor.clone())
        .extends(vec![project_anchor.clone()])
        .source("validator");

        locus_graph.store_event(master_event).await;

        // Step 3: Individual tool events
        for tool in &tools {
            let tool_ctx = format!("tool:{}", safe_context_name(&tool.name));
            let tool_event = CreateEventRequest::new(
                EventKind::Fact,
                serde_json::json!({
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                }),
            )
            .context_id(tool_ctx)
            .extends(vec![tool_anchor.clone()])
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
            .extends(vec![tool_anchor.clone()])
            .related_to(vec![project_anchor.clone()])
            .source("validator");

            locus_graph.store_event(meta_event).await;
        }
    });
}
