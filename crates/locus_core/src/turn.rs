use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::tool_call::{ToolResultData, ToolUse};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Thinking { thinking: String },
    ToolUse { tool_use: ToolUse },
    ToolResult { tool_result: ToolResultData },
    Error { error: String },
}

impl ContentBlock {
    pub fn text(content: impl Into<String>) -> Self {
        ContentBlock::Text {
            text: content.into(),
        }
    }

    pub fn thinking(content: impl Into<String>) -> Self {
        ContentBlock::Thinking {
            thinking: content.into(),
        }
    }

    pub fn tool_use(tool: ToolUse) -> Self {
        ContentBlock::ToolUse { tool_use: tool }
    }

    pub fn tool_result(result: ToolResultData) -> Self {
        ContentBlock::ToolResult { tool_result: result }
    }

    pub fn error(message: impl Into<String>) -> Self {
        ContentBlock::Error {
            error: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
}

impl TokenUsage {
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_read_tokens: None,
            cache_write_tokens: None,
        }
    }

    pub fn with_cache_read(mut self, tokens: u64) -> Self {
        self.cache_read_tokens = Some(tokens);
        self
    }

    pub fn with_cache_write(mut self, tokens: u64) -> Self {
        self.cache_write_tokens = Some(tokens);
        self
    }

    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub role: Role,
    pub blocks: Vec<ContentBlock>,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<TokenUsage>,
}

impl Turn {
    pub fn new(role: Role) -> Self {
        Self {
            role,
            blocks: Vec::new(),
            timestamp: Utc::now(),
            token_usage: None,
        }
    }

    pub fn with_block(mut self, block: ContentBlock) -> Self {
        self.blocks.push(block);
        self
    }

    pub fn with_token_usage(mut self, usage: TokenUsage) -> Self {
        self.token_usage = Some(usage);
        self
    }

    pub fn user() -> Self {
        Self::new(Role::User)
    }

    pub fn assistant() -> Self {
        Self::new(Role::Assistant)
    }

    pub fn system() -> Self {
        Self::new(Role::System)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_serialization() {
        let role = Role::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");

        let decoded: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, Role::User);
    }

    #[test]
    fn test_all_roles() {
        let roles = vec![Role::User, Role::Assistant, Role::System, Role::Tool];
        for role in roles {
            let json = serde_json::to_string(&role).unwrap();
            let decoded: Role = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, role);
        }
    }

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::text("hello world");
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains(r#""type":"text"#));
        assert!(json.contains("hello world"));

        let decoded: ContentBlock = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, ContentBlock::Text { .. }));
    }

    #[test]
    fn test_content_block_thinking() {
        let block = ContentBlock::thinking("let me think...");
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains(r#""type":"thinking"#));
    }

    #[test]
    fn test_content_block_error() {
        let block = ContentBlock::error("something went wrong");
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains(r#""type":"error"#));
    }

    #[test]
    fn test_token_usage_new() {
        let usage = TokenUsage::new(100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.total(), 150);
        assert_eq!(usage.cache_read_tokens, None);
    }

    #[test]
    fn test_token_usage_with_cache() {
        let usage = TokenUsage::new(100, 50)
            .with_cache_read(80)
            .with_cache_write(20);
        assert_eq!(usage.cache_read_tokens, Some(80));
        assert_eq!(usage.cache_write_tokens, Some(20));
    }

    #[test]
    fn test_token_usage_serialization() {
        let usage = TokenUsage::new(100, 50);
        let json = serde_json::to_string(&usage).unwrap();
        assert!(!json.contains("cache_read_tokens"));

        let usage_with_cache = TokenUsage::new(100, 50).with_cache_read(80);
        let json = serde_json::to_string(&usage_with_cache).unwrap();
        assert!(json.contains("cache_read_tokens"));
    }

    #[test]
    fn test_turn_new() {
        let turn = Turn::user();
        assert_eq!(turn.role, Role::User);
        assert!(turn.blocks.is_empty());
        assert!(turn.token_usage.is_none());
    }

    #[test]
    fn test_turn_with_block() {
        let turn = Turn::assistant()
            .with_block(ContentBlock::text("hello"))
            .with_block(ContentBlock::thinking("thinking..."));
        assert_eq!(turn.blocks.len(), 2);
    }

    #[test]
    fn test_turn_with_token_usage() {
        let turn = Turn::assistant()
            .with_token_usage(TokenUsage::new(100, 50));
        assert!(turn.token_usage.is_some());
        assert_eq!(turn.token_usage.unwrap().total(), 150);
    }

    #[test]
    fn test_turn_serialization() {
        let turn = Turn::user()
            .with_block(ContentBlock::text("hello world"))
            .with_token_usage(TokenUsage::new(10, 5));

        let json = serde_json::to_string(&turn).unwrap();
        let decoded: Turn = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.role, Role::User);
        assert_eq!(decoded.blocks.len(), 1);
        assert!(decoded.token_usage.is_some());
    }

    #[test]
    fn test_turn_factory_methods() {
        let user = Turn::user();
        assert_eq!(user.role, Role::User);

        let assistant = Turn::assistant();
        assert_eq!(assistant.role, Role::Assistant);

        let system = Turn::system();
        assert_eq!(system.role, Role::System);
    }
}
