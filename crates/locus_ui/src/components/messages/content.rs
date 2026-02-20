//! Content block within a message (text, thinking, tool use, tool result).

use super::tool::ToolDisplay;

/// Content block within a message.
#[derive(Debug, Clone)]
pub enum ContentBlock {
    /// Plain text content.
    Text(String),
    /// Thinking/reasoning block (collapsible).
    Thinking { text: String, expanded: bool },
    /// Tool use display.
    ToolUse(ToolDisplay),
    /// Tool result.
    ToolResult {
        tool_id: String,
        output: String,
        is_error: bool,
    },
}
