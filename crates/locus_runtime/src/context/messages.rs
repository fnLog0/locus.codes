//! Session-to-LLM message conversion and request building.

use locus_core::{ContentBlock, Session, Turn};
use locus_llms::types::{GenerateOptions, Message, Role as LlmRole, Tool, ToolChoice};
use locus_toolbus::ToolInfo;

use super::extract::extract_recent_files;

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

#[cfg(test)]
mod tests {
    use super::*;
    use locus_core::SessionConfig;

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
    fn test_turn_to_message() {
        let turn = Turn::user().with_block(ContentBlock::text("Hello world"));

        let msg = turn_to_message(&turn);

        assert!(msg.is_some());
        let msg = msg.unwrap();
        assert!(matches!(msg.role, LlmRole::User));
        let text = msg.text().unwrap();
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn test_turn_to_message_empty() {
        let turn = Turn::user();

        let msg = turn_to_message(&turn);

        assert!(msg.is_none());
    }
}
