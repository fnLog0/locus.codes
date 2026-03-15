//! Graph map — walk the hierarchy 2 levels deep from the project root.
//!
//! Gives the LLM structural awareness of what branches exist in LocusGraph
//! without pulling individual turn events (semantic search handles those).

use locus_graph::LocusGraphClient;
use tracing::warn;

/// Fetch children of a context_id via `extends` incoming links.
///
/// Returns `(context_id, context_type, context_name)` tuples.
async fn get_children(
    locus_graph: &LocusGraphClient,
    context_type: &str,
    context_name: &str,
) -> Vec<(String, String, String)> {
    match locus_graph
        .get_context_relationships(
            context_type,
            context_name,
            Some("extends"),
            Some("incoming"),
            None,
            Some(100),
        )
        .await
    {
        Ok(rels) => rels
            .into_iter()
            .filter_map(|r| {
                r.context
                    .map(|c| (c.context_id, c.context_type, c.context_name))
            })
            .collect(),
        Err(e) => {
            warn!(
                "Graph map: failed to get children for {}:{}: {}",
                context_type, context_name, e
            );
            vec![]
        }
    }
}

/// Split a `type:name` context_id into `(type, name)`.
fn split_context_id(ctx_id: &str) -> Option<(&str, &str)> {
    ctx_id.split_once(':')
}

/// Build a graph map string by walking 2 levels deep from the project root.
///
/// Produces a tree like:
/// ```text
/// project:locuscodes_abc123
///   ├── tool_anchor:locuscodes_abc123
///   │     ├── tool:bash
///   │     ├── tool:edit_file
///   │     └── meta:tool_search
///   ├── session_anchor:locuscodes_abc123
///   │     ├── session:fix-jwt_a1b2c3d4
///   │     └── session:add-mcp_e5f6g7h8
///   └── knowledge_anchor:locuscodes_abc123
/// ```
///
/// Turn-level events are excluded (too granular — semantic search handles those).
pub async fn build_graph_map(
    locus_graph: &LocusGraphClient,
    project_name: &str,
    repo_hash: &str,
) -> String {
    let root_id = super::project_anchor_id(project_name, repo_hash);
    let (root_type, root_name) = match split_context_id(&root_id) {
        Some(pair) => pair,
        None => return String::new(),
    };

    // Level 1: direct children of project root
    let level1 = get_children(locus_graph, root_type, root_name).await;
    if level1.is_empty() {
        return String::new();
    }

    let mut lines = vec![root_id.clone()];

    for (i, (child_id, child_type, child_name)) in level1.iter().enumerate() {
        let is_last = i == level1.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let prefix = if is_last { "      " } else { "│     " };

        lines.push(format!("  {}{}", connector, child_id));

        // Level 2: children of each level-1 node (skip turns — too granular)
        if child_type == "turn" {
            continue;
        }

        let level2 = get_children(locus_graph, child_type, child_name).await;

        // Filter out turn events from level 2 results
        let level2: Vec<_> = level2
            .into_iter()
            .filter(|(_, ctx_type, _)| ctx_type != "turn")
            .collect();

        for (j, (grandchild_id, _, _)) in level2.iter().enumerate() {
            let is_last_child = j == level2.len() - 1;
            let child_connector = if is_last_child {
                "└── "
            } else {
                "├── "
            };
            lines.push(format!(
                "  {}  {}{}",
                prefix, child_connector, grandchild_id
            ));
        }
    }

    lines.join("\n")
}
