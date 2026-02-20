use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::turn::Turn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Waiting,
    Running,
    Completed,
    Failed { error: String },
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub allowed_paths: Vec<PathBuf>,
    pub command_timeout_secs: u64,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            allowed_paths: Vec::new(),
            command_timeout_secs: 60,
        }
    }
}

impl SandboxPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_allowed_path(mut self, path: PathBuf) -> Self {
        self.allowed_paths.push(path);
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.command_timeout_secs = secs;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub model: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    pub sandbox_policy: SandboxPolicy,
}

impl SessionConfig {
    pub fn new(model: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            provider: provider.into(),
            max_turns: None,
            sandbox_policy: SandboxPolicy::default(),
        }
    }

    pub fn with_max_turns(mut self, max: u32) -> Self {
        self.max_turns = Some(max);
        self
    }

    pub fn with_sandbox_policy(mut self, policy: SandboxPolicy) -> Self {
        self.sandbox_policy = policy;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub status: SessionStatus,
    pub repo_root: PathBuf,
    pub config: SessionConfig,
    pub turns: Vec<Turn>,
    pub created_at: DateTime<Utc>,
}

impl Session {
    pub fn new(repo_root: PathBuf, config: SessionConfig) -> Self {
        Self {
            id: SessionId::new(),
            status: SessionStatus::Active,
            repo_root,
            config,
            turns: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_turn(&mut self, turn: Turn) {
        self.turns.push(turn);
    }

    pub fn set_status(&mut self, status: SessionStatus) {
        self.status = status;
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, SessionStatus::Active | SessionStatus::Running)
    }

    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_new() {
        let id = SessionId::new();
        assert!(!id.0.is_empty());
        assert_eq!(id.as_str().len(), 36); // UUID format
    }

    #[test]
    fn test_session_id_display() {
        let id = SessionId::new();
        let display = format!("{}", id);
        assert_eq!(display, id.as_str());
    }

    #[test]
    fn test_session_id_serialization() {
        let id = SessionId::new();
        let json = serde_json::to_string(&id).unwrap();
        let decoded: SessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.0, id.0);
    }

    #[test]
    fn test_session_status_active() {
        let status = SessionStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#"{"type":"active"}"#);
    }

    #[test]
    fn test_session_status_failed() {
        let status = SessionStatus::Failed {
            error: "something went wrong".to_string(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains(r#""type":"failed"#));
        assert!(json.contains("something went wrong"));
    }

    #[test]
    fn test_all_session_statuses() {
        let statuses = vec![
            SessionStatus::Active,
            SessionStatus::Waiting,
            SessionStatus::Running,
            SessionStatus::Completed,
            SessionStatus::Failed {
                error: "test".to_string(),
            },
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let decoded: SessionStatus = serde_json::from_str(&json).unwrap();
            assert!(matches!(decoded, _ if format!("{:?}", decoded) == format!("{:?}", status)));
        }
    }

    #[test]
    fn test_sandbox_policy_default() {
        let policy = SandboxPolicy::default();
        assert!(policy.allowed_paths.is_empty());
        assert_eq!(policy.command_timeout_secs, 60);
    }

    #[test]
    fn test_sandbox_policy_builder() {
        let policy = SandboxPolicy::new()
            .with_allowed_path(PathBuf::from("/tmp"))
            .with_timeout(120);
        assert_eq!(policy.allowed_paths.len(), 1);
        assert_eq!(policy.command_timeout_secs, 120);
    }

    #[test]
    fn test_session_config_new() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        assert_eq!(config.model, "claude-sonnet-4");
        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.max_turns, None);
    }

    #[test]
    fn test_session_config_with_max_turns() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic").with_max_turns(10);
        assert_eq!(config.max_turns, Some(10));
    }

    #[test]
    fn test_session_new() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let session = Session::new(PathBuf::from("/repo"), config.clone());
        
        assert!(!session.id.0.is_empty());
        assert_eq!(session.status, SessionStatus::Active);
        assert_eq!(session.repo_root, PathBuf::from("/repo"));
        assert_eq!(session.config.model, config.model);
        assert!(session.turns.is_empty());
    }

    #[test]
    fn test_session_add_turn() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let mut session = Session::new(PathBuf::from("/repo"), config);
        
        session.add_turn(Turn::user().with_block(crate::turn::ContentBlock::text("hello")));
        assert_eq!(session.turn_count(), 1);
    }

    #[test]
    fn test_session_set_status() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let mut session = Session::new(PathBuf::from("/repo"), config);
        
        // Running status should be considered active
        session.set_status(SessionStatus::Running);
        assert_eq!(session.status, SessionStatus::Running);
        assert!(session.is_active());
        
        // Completed status should NOT be active
        session.set_status(SessionStatus::Completed);
        assert!(!session.is_active());
    }

    #[test]
    fn test_session_serialization() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let session = Session::new(PathBuf::from("/repo"), config);
        
        let json = serde_json::to_string(&session).unwrap();
        let decoded: Session = serde_json::from_str(&json).unwrap();
        
        assert_eq!(decoded.id.0, session.id.0);
        assert_eq!(decoded.repo_root, session.repo_root);
    }
}
