//! OpenAI-specific types

use serde::{Deserialize, Serialize};

/// Configuration for OpenAI provider
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL (default: https://api.openai.com/v1)
    pub base_url: String,
    /// Organization ID (optional)
    pub organization_id: Option<String>,
    /// Project ID (optional, for project-scoped API keys)
    pub project_id: Option<String>,
}

impl OpenAIConfig {
    /// Create new config with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            organization_id: None,
            project_id: None,
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

    /// Set organization ID
    pub fn with_organization_id(mut self, org_id: impl Into<String>) -> Self {
        self.organization_id = Some(org_id.into());
        self
    }

    /// Set project ID
    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self::new(std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| String::new()))
    }
}

/// OpenAI chat completion request
#[derive(Debug, Serialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAITool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

/// OpenAI message
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: OpenAIMessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// OpenAI message content (can be string or array of content blocks)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum OpenAIMessageContent {
    String(String),
    Array(Vec<OpenAIContentBlock>),
}

/// OpenAI content block
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OpenAIContentBlock {
    Text {
        text: String,
    },
    ImageUrl {
        image_url: OpenAIImageUrl,
    },
}

/// OpenAI image URL
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAIImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// OpenAI tool definition
#[derive(Debug, Serialize, Clone)]
pub struct OpenAITool {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: OpenAIFunction,
}

/// OpenAI function definition
#[derive(Debug, Serialize, Clone)]
pub struct OpenAIFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// OpenAI tool call
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAIToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub function: OpenAIFunctionCall,
}

/// OpenAI function call
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAIFunctionCall {
    pub name: String,
    pub arguments: String,
}

/// OpenAI chat completion response
#[derive(Debug, Deserialize)]
pub struct OpenAIResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: OpenAIUsage,
}

/// OpenAI choice
#[derive(Debug, Deserialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub finish_reason: Option<String>,
}

/// OpenAI usage statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_hit_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_miss_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
}

/// OpenAI streaming event
#[derive(Debug, Deserialize)]
pub struct OpenAIStreamEvent {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<u64>,
    pub model: Option<String>,
    pub choices: Option<Vec<OpenAIStreamChoice>>,
    pub usage: Option<OpenAIUsage>,
    #[serde(skip)]
    pub error: Option<OpenAIError>,
}

/// OpenAI streaming choice
#[derive(Debug, Deserialize)]
pub struct OpenAIStreamChoice {
    pub index: u32,
    pub delta: OpenAIStreamDelta,
    pub finish_reason: Option<String>,
}

/// OpenAI streaming delta
#[derive(Debug, Deserialize, Clone)]
pub struct OpenAIStreamDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<OpenAIStreamToolCall>>,
}

/// OpenAI streaming tool call
#[derive(Debug, Deserialize, Clone)]
pub struct OpenAIStreamToolCall {
    pub index: u32,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub function: Option<OpenAIStreamFunctionCall>,
}

/// OpenAI streaming function call
#[derive(Debug, Deserialize, Clone)]
pub struct OpenAIStreamFunctionCall {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

/// OpenAI error
#[derive(Debug, Deserialize)]
pub struct OpenAIError {
    pub message: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Infer max_completion_tokens based on model name
pub fn infer_max_tokens(model: &str) -> u32 {
    if model.contains("o1") || model.contains("o3") {
        100000
    } else if model.contains("gpt-4") {
        8192
    } else {
        4096
    }
}
