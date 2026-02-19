//! Z.AI-specific types

use serde::{Deserialize, Serialize};

/// Configuration for Z.AI provider
#[derive(Debug, Clone)]
pub struct ZaiConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL (default: https://api.z.ai/api/paas/v4/)
    pub base_url: String,
}

impl ZaiConfig {
    /// Create new config with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.z.ai/api/paas/v4/".to_string(),
        }
    }

    /// Set base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        let mut url = base_url.into();
        if !url.ends_with('/') {
            url.push('/');
        }
        self.base_url = url;
        self
    }
}

impl Default for ZaiConfig {
    fn default() -> Self {
        Self::new(std::env::var("ZAI_API_KEY").unwrap_or_default())
    }
}

/// Z.AI chat completion request
#[derive(Debug, Serialize)]
pub struct ZaiRequest {
    pub model: String,
    pub messages: Vec<ZaiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ZaiThinkingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_stream: Option<bool>,
}

/// Thinking/reasoning configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZaiThinkingConfig {
    #[serde(rename = "type")]
    pub type_: String,
}

/// Z.AI message
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZaiMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ZaiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Z.AI tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaiToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub function: ZaiFunction,
}

/// Z.AI function in a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaiFunction {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Z.AI chat completion response
#[derive(Debug, Deserialize)]
pub struct ZaiResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ZaiChoice>,
    pub usage: ZaiUsage,
}

/// Z.AI response choice
#[derive(Debug, Deserialize)]
pub struct ZaiChoice {
    pub index: u32,
    pub message: ZaiResponseMessage,
    pub finish_reason: Option<String>,
}

/// Z.AI response message
#[derive(Debug, Deserialize)]
pub struct ZaiResponseMessage {
    pub role: String,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<ZaiToolCall>>,
}

/// Z.AI usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<ZaiPromptTokensDetails>,
}

/// Z.AI prompt tokens details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaiPromptTokensDetails {
    pub cached_tokens: Option<u32>,
}

/// Z.AI streaming chunk
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ZaiStreamChunk {
    pub id: String,
    pub model: String,
    pub choices: Vec<ZaiStreamChoice>,
    #[serde(default)]
    pub usage: Option<ZaiUsage>,
}

/// Z.AI streaming choice
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ZaiStreamChoice {
    pub index: u32,
    pub delta: ZaiDelta,
    pub finish_reason: Option<String>,
}

/// Z.AI streaming delta
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ZaiDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<ZaiStreamToolCall>>,
}

/// Z.AI streaming tool call (partial)
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ZaiStreamToolCall {
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub function: ZaiStreamFunction,
}

/// Z.AI streaming function (partial)
#[derive(Debug, Clone, Deserialize)]
pub struct ZaiStreamFunction {
    pub name: Option<String>,
    pub arguments: Option<String>,
}
