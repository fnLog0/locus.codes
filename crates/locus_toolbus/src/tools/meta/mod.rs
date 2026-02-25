//! Meta-tool definitions for tool discovery and sub-agents.
//!
//! These are not ToolBus-executed tools — they are handled by the Runtime.
//! They live here so all tool schemas are co-located and available via
//! `locus_toolbus::meta_tool_definitions()`.

use crate::ToolInfo;

/// Returns meta-tool definitions for tool discovery and sub-agents.
pub fn meta_tool_definitions() -> Vec<ToolInfo> {
    vec![
        ToolInfo {
            name: "tool_search".to_string(),
            description: "Search for available tools by describing what you want to do. Returns tool names and summaries.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Describe what you want to do, e.g. 'create a GitHub pull request'"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum results to return (default: 5)"
                    }
                },
                "required": ["query"]
            }),
        },
        ToolInfo {
            name: "tool_explain".to_string(),
            description: "Get the full schema for a specific tool before calling it. Use after tool_search.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "tool_id": {
                        "type": "string",
                        "description": "The tool name from tool_search results"
                    }
                },
                "required": ["tool_id"]
            }),
        },
        task_tool_definition(),
    ]
}

/// Task tool definition for the LLM.
pub fn task_tool_definition() -> ToolInfo {
    ToolInfo {
        name: "task".to_string(),
        description: "Run a sub-task in a separate agent. Use for independent, parallelizable work. Multiple task calls in the same response run in parallel. Do NOT use for simple single-file edits.".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Detailed instructions for the sub-agent. Include all necessary context — the sub-agent cannot see the parent conversation."
                },
                "description": {
                    "type": "string",
                    "description": "Short label for this task (shown in UI)"
                }
            },
            "required": ["prompt", "description"]
        }),
    }
}
