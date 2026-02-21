use thiserror::Error;

/// MCP-related errors
#[derive(Error, Debug)]
pub enum McpError {
    #[error("MCP server not found: {0}")]
    ServerNotFound(String),

    #[error("MCP server already running: {0}")]
    ServerAlreadyRunning(String),

    #[error("MCP server not running: {0}")]
    ServerNotRunning(String),

    #[error("Failed to start MCP server: {0}")]
    StartFailed(String),

    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type alias for MCP operations
pub type McpResult<T> = Result<T, McpError>;
