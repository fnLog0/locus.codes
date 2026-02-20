//! Context and prompt building for the Runtime.
//!
//! This module handles building prompts, managing context windows, and
//! converting session state into LLM-compatible message format.

use std::path::Path;

use locus_core::{ContentBlock, Session, SessionEvent, Turn};
use locus_graph::{InsightsOptions, LocusGraphClient};
use locus_llms::types::{GenerateOptions, Message, Role as LlmRole, Tool, ToolChoice};
use locus_toolbus::ToolInfo;

/// Meta-tool definitions for tool discovery and sub-agents.
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
use tokio::sync::mpsc;
use tracing::info;

use crate::error::RuntimeError;

/// Build the system prompt with tool descriptions.
///
/// Includes the agent identity, capabilities, and safety rules.
pub fn build_system_prompt(tools: &[ToolInfo]) -> String {
    let tools_desc = format_tools(tools);

    format!(
        r#"You are locus.codes, a terminal-native coding agent with persistent memory.

## Role
You help users write, refactor, debug, and understand code. You have access to
tools for file operations, command execution, and code search. You learn from
every interaction via LocusGraph memory.

## Tools Available
{tools_desc}

## Safety Rules
- Never run destructive commands without confirmation
- Never commit secrets to version control
- Always verify file paths before editing
- Use the bash tool with caution - it has full system access

## Memory
You have access to memories from previous sessions. Use them to:
- Maintain consistency with past decisions
- Learn from errors and solutions
- Remember project conventions and patterns
- Track user preferences

## Behavior
- Be concise and direct
- Make autonomous decisions when clear
- Ask for clarification only when truly ambiguous
- Store important decisions and outcomes to memory
"#
    )
}

/// Build session context string for the prompt.
///
/// Includes working directory, repo name, and recent activity.
pub fn build_session_context(session: &Session) -> String {
    let repo_name = session
        .repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let recent_files = extract_recent_files(session);

    format!(
        r#"## Current Session
- Working directory: {}
- Repository: {}
- Session ID: {}
- Turns completed: {}
- Files recently mentioned: {}
"#,
        session.repo_root.display(),
        repo_name,
        session.id.as_str(),
        session.turn_count(),
        recent_files.join(", ")
    )
}

/// Build messages array for the LLM request.
///
/// Converts session turns into the message format expected by the LLM,
/// prepending system prompt with session context and memories.
pub fn build_messages(
    system_prompt: &str,
    session: &Session,
    memories: &str,
) -> Vec<Message> {
    let mut messages = Vec::new();

    // System message with prompt
    let mut system_content = system_prompt.to_string();

    // Append session context
    system_content.push_str("\n\n");
    system_content.push_str(&build_session_context(session));

    // Append memories if any
    if !memories.is_empty() {
        system_content.push_str("\n\n## Relevant Memories\n");
        system_content.push_str(memories);
    }

    messages.push(Message::new(LlmRole::System, system_content));

    // Convert previous turns to messages
    for turn in &session.turns {
        if let Some(msg) = turn_to_message(turn) {
            messages.push(msg);
        }
    }

    messages
}

/// Build a GenerateRequest with all necessary configuration.
///
/// Creates a complete request ready to send to the LLM provider.
pub fn build_generate_request(
    model: &str,
    messages: Vec<Message>,
    tools: &[ToolInfo],
    max_tokens: u32,
) -> locus_llms::types::GenerateRequest {
    let llm_tools: Vec<Tool> = tools
        .iter()
        .map(|t| {
            Tool::function(&t.name, &t.description).parameters(t.parameters.clone())
        })
        .collect();

    let options = GenerateOptions::new()
        .max_tokens(max_tokens)
        .temperature(0.7);

    let options = if llm_tools.is_empty() {
        options
    } else {
        let mut opts = options;
        for tool in llm_tools {
            opts = opts.add_tool(tool);
        }
        opts.tool_choice(ToolChoice::Auto)
    };

    locus_llms::types::GenerateRequest {
        model: model.to_string(),
        messages,
        options,
        provider_options: None,
        telemetry_metadata: None,
    }
}

/// Check if the session is approaching context limit.
///
/// Uses a simple token estimation based on character count.
/// Returns true if estimated tokens exceed 85% of the limit.
pub fn near_context_limit(session: &Session, context_limit: u64) -> bool {
    let estimated_tokens = estimate_session_tokens(session);
    let threshold = (context_limit as f64 * 0.85) as u64;
    estimated_tokens > threshold
}

/// Estimate token count for a session.
///
/// Uses a rough heuristic of ~4 characters per token.
fn estimate_session_tokens(session: &Session) -> u64 {
    let mut char_count = 0usize;

    for turn in &session.turns {
        for block in &turn.blocks {
            match block {
                ContentBlock::Text { text } => char_count += text.len(),
                ContentBlock::Thinking { thinking } => char_count += thinking.len(),
                ContentBlock::Error { error } => char_count += error.len(),
                ContentBlock::ToolUse { tool_use } => {
                    char_count += tool_use.name.len();
                    char_count += tool_use.args.to_string().len();
                }
                ContentBlock::ToolResult { tool_result } => {
                    char_count += tool_result.output.to_string().len();
                }
            }
        }
    }

    // Rough estimate: ~4 characters per token
    (char_count / 4) as u64
}

/// Compress context when approaching limit.
///
/// Uses LocusGraph to generate a summary of the conversation and
/// replaces old turns with a summary turn.
pub async fn compress_context(
    locus_graph: &LocusGraphClient,
    session: &mut Session,
    event_tx: &mpsc::Sender<SessionEvent>,
) -> Result<(), RuntimeError> {
    info!("Compressing context for session {}", session.id.as_str());

    let _ = event_tx
        .send(SessionEvent::status("Context near limit, compressing..."))
        .await;

    // Build a summary prompt from the turns
    let turns_summary = summarize_turns(&session.turns);

    // Use LocusGraph insights to compress
    let options = InsightsOptions::new().limit(20);

    let insight_result = locus_graph
        .generate_insights(
            &format!(
                "Summarize this conversation, preserving key decisions and context:\n\n{}",
                turns_summary
            ),
            Some(options),
        )
        .await
        .map_err(|e| RuntimeError::MemoryFailed(e.to_string()))?;

    // Keep only the last few turns and prepend a summary
    let keep_count = session.turns.len().saturating_sub(3).max(1);
    let summary = insight_result.insight;

    // Create a summary turn
    let summary_turn = Turn::system()
        .with_block(ContentBlock::text(format!(
            "[Context Summary]\n{}",
            summary
        )));

    // Replace old turns with summary
    let recent_turns: Vec<Turn> = session.turns.drain(keep_count..).collect();
    session.turns.clear();
    session.turns.push(summary_turn);
    session.turns.extend(recent_turns);

    let _ = event_tx
        .send(SessionEvent::status(format!(
            "Context compressed. {} turns remaining.",
            session.turn_count()
        )))
        .await;

    Ok(())
}

/// Convert a Turn to an LLM Message.
///
/// Returns None if the turn has no meaningful content.
/// Preserves structured tool calls and results for proper API compatibility.
fn turn_to_message(turn: &Turn) -> Option<Message> {
    use locus_llms::types::ContentPart;

    let role = match turn.role {
        locus_core::Role::User => LlmRole::User,
        locus_core::Role::Assistant => LlmRole::Assistant,
        locus_core::Role::System => LlmRole::System,
        locus_core::Role::Tool => LlmRole::Tool,
    };

    // Build content parts, preserving structured tool data
    let mut parts: Vec<ContentPart> = Vec::new();

    for block in &turn.blocks {
        match block {
            ContentBlock::Text { text } => {
                parts.push(ContentPart::text(text.clone()));
            }
            ContentBlock::Thinking { thinking } => {
                // Include thinking as text with a marker
                parts.push(ContentPart::text(format!("[Thinking] {}", thinking)));
            }
            ContentBlock::Error { error } => {
                parts.push(ContentPart::text(format!("[Error] {}", error)));
            }
            ContentBlock::ToolUse { tool_use } => {
                // Preserve structured tool call with id, name, and arguments
                parts.push(ContentPart::tool_call(
                    tool_use.id.clone(),
                    tool_use.name.clone(),
                    tool_use.args.clone(),
                ));
            }
            ContentBlock::ToolResult { tool_result } => {
                // For tool results, we need to extract the tool_use_id
                // The output contains the result
                let tool_use_id = tool_result
                    .output
                    .get("tool_use_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                parts.push(ContentPart::tool_result(
                    tool_use_id,
                    tool_result.output.clone(),
                ));
            }
        }
    }

    if parts.is_empty() {
        return None;
    }

    Some(Message::new(role, parts))
}

/// Format tool descriptions for the system prompt.
fn format_tools(tools: &[ToolInfo]) -> String {
    if tools.is_empty() {
        return "No tools available.".to_string();
    }

    tools
        .iter()
        .map(|t| {
            format!(
                "- **{}**: {}",
                t.name,
                t.description.lines().next().unwrap_or("No description")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract recently mentioned files from session turns.
fn extract_recent_files(session: &Session) -> Vec<String> {
    let mut files = Vec::new();
    let file_keywords = ["file_path", "path", "file:"];

    for turn in session.turns.iter().rev().take(5) {
        for block in &turn.blocks {
            if let ContentBlock::Text { text } = block {
                for line in text.lines() {
                    for keyword in &file_keywords {
                        if line.contains(keyword) {
                            // Try to extract a path-like string
                            if let Some(path) = extract_path_from_line(line) {
                                if !files.contains(&path) {
                                    files.push(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    files.truncate(5);
    files
}

/// Try to extract a file path from a line of text.
fn extract_path_from_line(line: &str) -> Option<String> {
    // Look for common patterns like "file: src/main.rs" or "path = /foo/bar"
    let parts: Vec<&str> = line.split_whitespace().collect();
    for part in parts {
        // Heuristic: path-like strings contain / and don't start with special chars
        if part.contains('/') && !part.starts_with(|c: char| c.is_ascii_punctuation()) {
            // Clean up the path
            let cleaned = part.trim_matches(|c| c == '"' || c == '\'' || c == ',');
            if Path::new(cleaned).extension().is_some() || cleaned.contains('/') {
                return Some(cleaned.to_string());
            }
        }
    }
    None
}

/// Create a text summary of turns for compression.
fn summarize_turns(turns: &[Turn]) -> String {
    turns
        .iter()
        .map(|t| {
            let role = match t.role {
                locus_core::Role::User => "User",
                locus_core::Role::Assistant => "Assistant",
                locus_core::Role::System => "System",
                locus_core::Role::Tool => "Tool",
            };

            let content: String = t
                .blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            format!("**{}**: {}", role, content.chars().take(500).collect::<String>())
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use locus_core::SessionConfig;

    #[test]
    fn test_build_system_prompt() {
        let tools = vec![
            ToolInfo {
                name: "bash".to_string(),
                description: "Execute shell commands".to_string(),
                parameters: serde_json::json!({}),
            },
            ToolInfo {
                name: "read".to_string(),
                description: "Read file contents".to_string(),
                parameters: serde_json::json!({}),
            },
        ];

        let prompt = build_system_prompt(&tools);

        assert!(prompt.contains("locus.codes"));
        assert!(prompt.contains("bash"));
        assert!(prompt.contains("read"));
        assert!(prompt.contains("Safety Rules"));
    }

    #[test]
    fn test_build_session_context() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let session = Session::new(std::path::PathBuf::from("/home/user/myproject"), config);

        let context = build_session_context(&session);

        assert!(context.contains("myproject"));
        assert!(context.contains("/home/user/myproject"));
        assert!(context.contains("Turns completed: 0"));
    }

    #[test]
    fn test_build_messages_empty_session() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let session = Session::new(std::path::PathBuf::from("/repo"), config);
        let system_prompt = "You are a helpful assistant.";
        let memories = "Previous context...";

        let messages = build_messages(system_prompt, &session, memories);

        assert_eq!(messages.len(), 1); // System only
        assert!(matches!(messages[0].role, LlmRole::System));
    }

    #[test]
    fn test_build_messages_with_turns() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let mut session = Session::new(std::path::PathBuf::from("/repo"), config);

        session.add_turn(
            Turn::user().with_block(ContentBlock::text("First message")),
        );
        session.add_turn(
            Turn::assistant().with_block(ContentBlock::text("First response")),
        );

        let messages = build_messages("System prompt", &session, "");

        assert_eq!(messages.len(), 3); // System + 2 turns
    }

    #[test]
    fn test_near_context_limit_false() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let session = Session::new(std::path::PathBuf::from("/repo"), config);
        let limit = 100_000u64;

        assert!(!near_context_limit(&session, limit));
    }

    #[test]
    fn test_near_context_limit_true() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let mut session = Session::new(std::path::PathBuf::from("/repo"), config);

        // Add a large amount of text
        let large_text = "x".repeat(400_000);
        session.add_turn(Turn::user().with_block(ContentBlock::text(large_text)));

        let limit = 100_000u64;

        assert!(near_context_limit(&session, limit));
    }

    #[test]
    fn test_estimate_session_tokens() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let mut session = Session::new(std::path::PathBuf::from("/repo"), config);

        // 400 chars should be ~100 tokens
        session.add_turn(
            Turn::user().with_block(ContentBlock::text("x".repeat(400))),
        );

        let tokens = estimate_session_tokens(&session);

        assert_eq!(tokens, 100);
    }

    #[test]
    fn test_format_tools() {
        let tools = vec![ToolInfo {
            name: "bash".to_string(),
            description: "Execute commands".to_string(),
            parameters: serde_json::json!({}),
        }];

        let formatted = format_tools(&tools);

        assert!(formatted.contains("bash"));
        assert!(formatted.contains("Execute commands"));
    }

    #[test]
    fn test_format_tools_empty() {
        let tools: Vec<ToolInfo> = vec![];
        let formatted = format_tools(&tools);

        assert!(formatted.contains("No tools available"));
    }

    #[test]
    fn test_turn_to_message() {
        let turn = Turn::user().with_block(ContentBlock::text("Hello world"));

        let msg = turn_to_message(&turn);

        assert!(msg.is_some());
        let msg = msg.unwrap();
        assert!(matches!(msg.role, LlmRole::User));
        // Content is now Parts, not Text
        let text = msg.text().unwrap();
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn test_turn_to_message_empty() {
        let turn = Turn::user();

        let msg = turn_to_message(&turn);

        assert!(msg.is_none());
    }

    #[test]
    fn test_summarize_turns() {
        let turns = vec![
            Turn::user().with_block(ContentBlock::text("User message")),
            Turn::assistant().with_block(ContentBlock::text("Assistant response")),
        ];

        let summary = summarize_turns(&turns);

        assert!(summary.contains("User"));
        assert!(summary.contains("Assistant"));
        assert!(summary.contains("User message"));
        assert!(summary.contains("Assistant response"));
    }
}
