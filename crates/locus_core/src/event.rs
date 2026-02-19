use serde::{Deserialize, Serialize};

use crate::session::SessionStatus;
use crate::tool_call::{ToolResultData, ToolUse};
use crate::turn::Role;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    TurnStart { role: Role },

    TextDelta { text: String },

    ThinkingDelta { thinking: String },

    ToolStart { tool_use: ToolUse },

    ToolDone {
        tool_use_id: String,
        result: ToolResultData,
    },

    MemoryRecall {
        query: String,
        items_found: u64,
    },

    Status { message: String },

    TurnEnd,

    Error { error: String },

    SessionEnd { status: SessionStatus },
}

impl SessionEvent {
    pub fn turn_start(role: Role) -> Self {
        SessionEvent::TurnStart { role }
    }

    pub fn text_delta(text: impl Into<String>) -> Self {
        SessionEvent::TextDelta { text: text.into() }
    }

    pub fn thinking_delta(thinking: impl Into<String>) -> Self {
        SessionEvent::ThinkingDelta {
            thinking: thinking.into(),
        }
    }

    pub fn tool_start(tool_use: ToolUse) -> Self {
        SessionEvent::ToolStart { tool_use }
    }

    pub fn tool_done(tool_use_id: impl Into<String>, result: ToolResultData) -> Self {
        SessionEvent::ToolDone {
            tool_use_id: tool_use_id.into(),
            result,
        }
    }

    pub fn memory_recall(query: impl Into<String>, items_found: u64) -> Self {
        SessionEvent::MemoryRecall {
            query: query.into(),
            items_found,
        }
    }

    pub fn status(message: impl Into<String>) -> Self {
        SessionEvent::Status {
            message: message.into(),
        }
    }

    pub fn turn_end() -> Self {
        SessionEvent::TurnEnd
    }

    pub fn error(error: impl Into<String>) -> Self {
        SessionEvent::Error {
            error: error.into(),
        }
    }

    pub fn session_end(status: SessionStatus) -> Self {
        SessionEvent::SessionEnd { status }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionStatus;

    #[test]
    fn test_turn_start() {
        let event = SessionEvent::turn_start(Role::User);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"turn_start"#));
        assert!(json.contains(r#""role":"user"#));
    }

    #[test]
    fn test_text_delta() {
        let event = SessionEvent::text_delta("hello world");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"text_delta"#));
        assert!(json.contains("hello world"));
    }

    #[test]
    fn test_thinking_delta() {
        let event = SessionEvent::thinking_delta("let me think...");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"thinking_delta"#));
        assert!(json.contains("let me think..."));
    }

    #[test]
    fn test_tool_start() {
        let tool = ToolUse::new("t1", "bash", serde_json::json!({"command": "ls"}));
        let event = SessionEvent::tool_start(tool);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"tool_start"#));
        assert!(json.contains("bash"));
    }

    #[test]
    fn test_tool_done() {
        let result = ToolResultData::success(serde_json::json!({"stdout": "ok"}), 100);
        let event = SessionEvent::tool_done("tool-1", result);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"tool_done"#));
        assert!(json.contains("tool-1"));
        assert!(json.contains(r#""is_error":false"#));
    }

    #[test]
    fn test_memory_recall() {
        let event = SessionEvent::memory_recall("search query", 5);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"memory_recall"#));
        assert!(json.contains("search query"));
        assert!(json.contains(r#""items_found":5"#));
    }

    #[test]
    fn test_status() {
        let event = SessionEvent::status("compressing context...");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"status"#));
        assert!(json.contains("compressing context..."));
    }

    #[test]
    fn test_turn_end() {
        let event = SessionEvent::turn_end();
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, r#"{"type":"turn_end"}"#);
    }

    #[test]
    fn test_error() {
        let event = SessionEvent::error("something went wrong");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"error"#));
        assert!(json.contains("something went wrong"));
    }

    #[test]
    fn test_session_end_active() {
        let event = SessionEvent::session_end(SessionStatus::Completed);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"session_end"#));
        assert!(json.contains(r#""type":"completed"#));
    }

    #[test]
    fn test_session_end_failed() {
        let event = SessionEvent::session_end(SessionStatus::Failed {
            error: "timeout".to_string(),
        });
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"session_end"#));
        assert!(json.contains("timeout"));
    }

    #[test]
    fn test_all_event_types_serializable() {
        let events = vec![
            SessionEvent::turn_start(Role::User),
            SessionEvent::text_delta("text"),
            SessionEvent::thinking_delta("thinking"),
            SessionEvent::tool_start(ToolUse::new("t1", "bash", serde_json::json!({}))),
            SessionEvent::tool_done("t1", ToolResultData::success(serde_json::json!({}), 0)),
            SessionEvent::memory_recall("q", 0),
            SessionEvent::status("status"),
            SessionEvent::turn_end(),
            SessionEvent::error("err"),
            SessionEvent::session_end(SessionStatus::Completed),
        ];

        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let decoded: SessionEvent = serde_json::from_str(&json).unwrap();
            // Verify roundtrip works - just checking we can serialize and deserialize
            let _ = format!("{:?}", decoded);
        }
    }

    #[test]
    fn test_event_roundtrip() {
        let event = SessionEvent::text_delta("hello world");
        let json = serde_json::to_string(&event).unwrap();
        let decoded: SessionEvent = serde_json::from_str(&json).unwrap();
        
        if let SessionEvent::TextDelta { text } = decoded {
            assert_eq!(text, "hello world");
        } else {
            panic!("Expected TextDelta variant");
        }
    }
}
