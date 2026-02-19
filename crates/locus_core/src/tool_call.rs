use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
    pub status: ToolStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<PathBuf>,
}

impl ToolUse {
    pub fn new(id: impl Into<String>, name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            args,
            status: ToolStatus::Pending,
            file_path: None,
        }
    }

    pub fn with_file_path(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    pub fn with_status(mut self, status: ToolStatus) -> Self {
        self.status = status;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolStatus {
    Pending,
    Running,
    Done { result: ToolResultData },
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResultData {
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub is_error: bool,
}

impl ToolResultData {
    pub fn success(output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            output,
            duration_ms,
            is_error: false,
        }
    }

    pub fn error(output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            output,
            duration_ms,
            is_error: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_use_new() {
        let tool = ToolUse::new("tool-1", "bash", serde_json::json!({"command": "ls"}));
        assert_eq!(tool.id, "tool-1");
        assert_eq!(tool.name, "bash");
        assert_eq!(tool.status, ToolStatus::Pending);
        assert_eq!(tool.file_path, None);
    }

    #[test]
    fn test_tool_use_with_file_path() {
        let tool = ToolUse::new("tool-2", "edit_file", serde_json::json!({}))
            .with_file_path(PathBuf::from("/src/main.rs"));
        assert_eq!(tool.file_path, Some(PathBuf::from("/src/main.rs")));
    }

    #[test]
    fn test_tool_status_pending() {
        let status = ToolStatus::Pending;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#"{"type":"pending"}"#);
    }

    #[test]
    fn test_tool_status_running() {
        let status = ToolStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#"{"type":"running"}"#);
    }

    #[test]
    fn test_tool_status_done() {
        let result = ToolResultData::success(
            serde_json::json!({"stdout": "hello"}),
            100,
        );
        let status = ToolStatus::Done { result };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains(r#""type":"done"#));
        assert!(json.contains(r#""is_error":false"#));
    }

    #[test]
    fn test_tool_status_failed() {
        let status = ToolStatus::Failed { error: "command not found".to_string() };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains(r#""type":"failed"#));
        assert!(json.contains("command not found"));
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResultData::success(
            serde_json::json!({"files": ["a.rs", "b.rs"]}),
            50,
        );
        assert_eq!(result.duration_ms, 50);
        assert!(!result.is_error);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResultData::error(
            serde_json::json!({"error": "permission denied"}),
            10,
        );
        assert!(result.is_error);
    }

    #[test]
    fn test_tool_use_serialization() {
        let tool = ToolUse::new("t1", "grep", serde_json::json!({"pattern": "fn main"}))
            .with_status(ToolStatus::Running);

        let json = serde_json::to_string(&tool).unwrap();
        let decoded: ToolUse = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.id, "t1");
        assert_eq!(decoded.name, "grep");
        assert!(matches!(decoded.status, ToolStatus::Running));
    }

    #[test]
    fn test_tool_use_deserialization() {
        let json = serde_json::json!({
            "id": "tool-123",
            "name": "bash",
            "args": {"command": "echo hello"},
            "status": {"type": "pending"},
            "file_path": "/src/lib.rs"
        });

        let tool: ToolUse = serde_json::from_value(json).unwrap();
        assert_eq!(tool.id, "tool-123");
        assert_eq!(tool.name, "bash");
        assert_eq!(tool.file_path, Some(PathBuf::from("/src/lib.rs")));
    }
}
