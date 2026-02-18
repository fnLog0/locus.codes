//! Z.AI-specific types

use serde::{Deserialize, Serialize};

/// Configuration for Z.AI provider
#[derive(Debug, Clone)]
pub struct ZiaiConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL (default: https://api.z.ai/api/paas/v4/)
    pub base_url: String,
}

impl ZiaiConfig {
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

impl Default for ZiaiConfig {
    fn default() -> Self {
        Self::new(std::env::var("ZAI_API_KEY").unwrap_or_default())
    }
}

/// Z.AI chat completion request
#[derive(Debug, Serialize)]
pub struct ZiaiRequest {
    pub model: String,
    pub messages: Vec<ZiaiMessage>,
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
    pub thinking: Option<ZiaiThinkingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_stream: Option<bool>,
}

/// Thinking/reasoning configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZiaiThinkingConfig {
    #[serde(rename = "type")]
    pub type_: String,
}

/// Z.AI message
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZiaiMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ZiaiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Z.AI tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiaiToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub function: ZiaiFunction,
}

/// Z.AI function in a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiaiFunction {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Z.AI chat completion response
#[derive(Debug, Deserialize)]
pub struct ZiaiResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ZiaiChoice>,
    pub usage: ZiaiUsage,
}

/// Z.AI response choice
#[derive(Debug, Deserialize)]
pub struct ZiaiChoice {
    pub index: u32,
    pub message: ZiaiResponseMessage,
    pub finish_reason: Option<String>,
}

/// Z.AI response message
#[derive(Debug, Deserialize)]
pub struct ZiaiResponseMessage {
    pub role: String,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<ZiaiToolCall>>,
}

/// Z.AI usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiaiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<ZiaiPromptTokensDetails>,
}

/// Z.AI prompt tokens details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiaiPromptTokensDetails {
    pub cached_tokens: Option<u32>,
}

/// Z.AI streaming chunk
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ZiaiStreamChunk {
    pub id: String,
    pub model: String,
    pub choices: Vec<ZiaiStreamChoice>,
    #[serde(default)]
    pub usage: Option<ZiaiUsage>,
}

/// Z.AI streaming choice
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ZiaiStreamChoice {
    pub index: u32,
    pub delta: ZiaiDelta,
    pub finish_reason: Option<String>,
}

/// Z.AI streaming delta
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ZiaiDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<ZiaiStreamToolCall>>,
}

/// Z.AI streaming tool call (partial)
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ZiaiStreamToolCall {
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub function: ZiaiStreamFunction,
}

/// Z.AI streaming function (partial)
#[derive(Debug, Clone, Deserialize)]
pub struct ZiaiStreamFunction {
    pub name: Option<String>,
    pub arguments: Option<String>,
}
