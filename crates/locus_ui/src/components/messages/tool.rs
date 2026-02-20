//! Tool execution display and status.

use std::time::Duration;

/// Tool execution display.
#[derive(Debug, Clone)]
pub struct ToolDisplay {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
    pub status: ToolStatus,
    pub output: Option<String>,
    pub duration: Option<Duration>,
}

/// Tool execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Done,
    Error,
}

impl ToolStatus {
    /// Get the status indicator symbol.
    pub fn indicator(&self) -> &'static str {
        match self {
            ToolStatus::Running => "*",
            ToolStatus::Done => "+",
            ToolStatus::Error => "!",
        }
    }
}
