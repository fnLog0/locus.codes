//! Runtime error types

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Tool '{tool}' failed: {message}")]
    ToolFailed { tool: String, message: String },

    #[error("LLM error: {0}")]
    LlmFailed(String),

    #[error("Context overflow - token limit exceeded")]
    ContextOverflow,

    #[error("Memory operation failed: {0}")]
    MemoryFailed(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;
