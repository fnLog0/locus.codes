//! Request/response types (docs 06_llm_engine/response_schema.md).

use serde::Deserialize;
use locus_core::Mode;

/// Mode-specific limits for LLM calls
#[derive(Debug, Clone, Copy)]
pub struct ModeLimits {
    /// Maximum input tokens (system + context + user)
    pub max_input_tokens: usize,
    /// Maximum output tokens
    pub max_output_tokens: usize,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retries: usize,
}

impl ModeLimits {
    pub fn for_mode(mode: Mode) -> Self {
        match mode {
            Mode::Rush => Self {
                max_input_tokens: 6000,
                max_output_tokens: 2000,
                timeout_secs: 30,
                max_retries: 1,
            },
            Mode::Smart => Self {
                max_input_tokens: 24000,
                max_output_tokens: 8000,
                timeout_secs: 120,
                max_retries: 3,
            },
            Mode::Deep => Self {
                max_input_tokens: 48000,
                max_output_tokens: 16000,
                timeout_secs: 300,
                max_retries: 5,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub system_prompt: String,
    pub memory_bundle: String,
    pub tool_definitions: String,
    pub user_prompt: String,
    /// Mode limits for this request
    pub limits: ModeLimits,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompletionResponse {
    pub reasoning: String,
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub confidence: Option<f32>,
}
