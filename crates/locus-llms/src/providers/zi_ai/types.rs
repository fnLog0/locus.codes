//! Zi.AI-specific types

use serde::{Deserialize, Serialize};

/// Configuration for Zi.AI provider
#[derive(Debug, Clone)]
pub struct ZiAIConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL (default: https://api.z.ai/v1)
    pub base_url: String,
    /// Model ID (default: zai)
    pub model: String,
}

impl ZiAIConfig {
    /// Create new config with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.z.ai/v1".to_string(),
            model: "zai".to_string(),
        }
    }

    /// Set base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        let mut url = base_url.into();
        // Strip /chat/completions suffix if user provided full endpoint URL
        if url.ends_with("/chat/completions") {
            url = url.trim_end_matches("/chat/completions").to_string();
        } else if url.ends_with("/chat/completions/") {
            url = url.trim_end_matches("/chat/completions/").to_string();
        }
        // Ensure URL doesn't end with /
        if url.ends_with('/') {
            url = url.trim_end_matches('/').to_string();
        }
        self.base_url = url;
        self
    }

    /// Set model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

impl Default for ZiAIConfig {
    fn default() -> Self {
        Self::new(std::env::var("ZI_AI_API_KEY").unwrap_or_else(|_| String::new()))
    }
}

/// Zi.AI chat completion request
#[derive(Debug, Serialize)]
pub struct ZiAIRequest {
    pub model: String,
    pub messages: Vec<ZiAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ZiAITool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
}

/// Zi.AI message
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZiAIMessage {
    pub role: String,
    pub content: ZiAIMessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ZiAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Zi.AI message content (can be string or array of content blocks)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ZiAIMessageContent {
    String(String),
    Array(Vec<ZiAIContentBlock>),
}

/// Zi.AI content block
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ZiAIContentBlock {
    Text {
        text: String,
    },
    ImageUrl {
        image_url: ZiAIImageUrl,
    },
}

/// Zi.AI image URL
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZiAIImageUrl {
    pub url: String,
}

/// Zi.AI tool definition
#[derive(Debug, Serialize, Clone)]
pub struct ZiAITool {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: ZiAIFunction,
}

/// Zi.AI function definition
#[derive(Debug, Serialize, Clone)]
pub struct ZiAIFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Zi.AI tool call
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZiAIToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub function: ZiAIFunctionCall,
}

/// Zi.AI function call
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZiAIFunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Zi.AI chat completion response
#[derive(Debug, Deserialize)]
pub struct ZiAIResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ZiAIChoice>,
    pub usage: ZiAIUsage,
}

/// Zi.AI choice
#[derive(Debug, Deserialize)]
pub struct ZiAIChoice {
    pub index: u32,
    pub message: ZiAIMessage,
    pub finish_reason: Option<String>,
}

/// Zi.AI usage statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZiAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Zi.AI streaming event
#[derive(Debug, Deserialize)]
pub struct ZiAIStreamEvent {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<u64>,
    pub model: Option<String>,
    pub choices: Option<Vec<ZiAIStreamChoice>>,
    pub usage: Option<ZiAIUsage>,
}

/// Zi.AI streaming choice
#[derive(Debug, Deserialize)]
pub struct ZiAIStreamChoice {
    pub index: u32,
    pub delta: ZiAIStreamDelta,
    pub finish_reason: Option<String>,
}

/// Zi.AI streaming delta
#[derive(Debug, Deserialize, Clone)]
pub struct ZiAIStreamDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ZiAIStreamToolCall>>,
}

/// Zi.AI streaming tool call
#[derive(Debug, Deserialize, Clone)]
pub struct ZiAIStreamToolCall {
    pub index: u32,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub function: Option<ZiAIStreamFunctionCall>,
}

/// Zi.AI streaming function call
#[derive(Debug, Deserialize, Clone)]
pub struct ZiAIStreamFunctionCall {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

/// Infer max_tokens based on model name
pub fn infer_max_tokens(_model: &str) -> u32 {
    4096
}
