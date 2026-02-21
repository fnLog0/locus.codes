use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::turn::{ContentBlock, Role, Turn};

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
    Cancelled,
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

/// Identifies a prior session when extending to a new one (continuity).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentSessionId(pub String);

impl ParentSessionId {
    pub fn from_session_id(id: &SessionId) -> Self {
        Self(id.0.clone())
    }

    pub fn as_str(&self) -> &str {
        &self.0
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
    /// When the current run started (set at run start, cleared at run end).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_started_at: Option<DateTime<Utc>>,
    /// Duration of the last completed run in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_duration_ms: Option<u64>,
    /// If this session was created by "extend", the parent session id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<ParentSessionId>,
    /// Total prompt (input) tokens consumed across all LLM calls in this session.
    #[serde(default)]
    pub total_prompt_tokens: u64,
    /// Total completion (output) tokens consumed across all LLM calls in this session.
    #[serde(default)]
    pub total_completion_tokens: u64,
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
            run_started_at: None,
            last_run_duration_ms: None,
            parent_session_id: None,
            total_prompt_tokens: 0,
            total_completion_tokens: 0,
        }
    }

    /// Create a new session that continues from a previous one (same repo and config, new id).
    /// Sets `parent_session_id` so the runtime/LocusGraph can link context across sessions.
    pub fn new_continuing(prev: &Session) -> Self {
        let mut next = Self::new(prev.repo_root.clone(), prev.config.clone());
        next.parent_session_id = Some(ParentSessionId::from_session_id(&prev.id));
        next
    }

    /// Mark the start of a run (for duration tracking). Call when `run()` starts.
    pub fn start_run(&mut self) {
        self.run_started_at = Some(Utc::now());
    }

    /// Mark the end of a run and record duration. Call when `run()` completes.
    /// If `start_run()` was used, duration can be computed from `run_started_at`;
    /// otherwise pass the duration explicitly.
    pub fn finish_run(&mut self, duration_ms: Option<u64>) {
        let ms = duration_ms.or_else(|| {
            self.run_started_at.map(|start| {
                let elapsed = Utc::now() - start;
                elapsed.num_milliseconds().max(0) as u64
            })
        });
        self.last_run_duration_ms = ms;
        self.run_started_at = None;
    }

    /// Duration of the last completed run in milliseconds, if any.
    pub fn last_run_duration_ms(&self) -> Option<u64> {
        self.last_run_duration_ms
    }

    /// Record token usage from an LLM call (updates session totals).
    pub fn add_llm_usage(&mut self, prompt_tokens: u64, completion_tokens: u64) {
        self.total_prompt_tokens = self.total_prompt_tokens.saturating_add(prompt_tokens);
        self.total_completion_tokens = self.total_completion_tokens.saturating_add(completion_tokens);
    }

    /// Total tokens (prompt + completion) consumed in this session.
    pub fn total_tokens(&self) -> u64 {
        self.total_prompt_tokens.saturating_add(self.total_completion_tokens)
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

    /// Build a summary of this session for display or logging at session end.
    pub fn build_summary(&self) -> SessionSummary {
        let mut tools_used: Vec<String> = Vec::new();
        let mut first_user_message: Option<String> = None;

        for turn in &self.turns {
            if first_user_message.is_none() && turn.role == Role::User {
                for block in &turn.blocks {
                    if let ContentBlock::Text { text } = block {
                        let preview = if text.len() > 120 {
                            format!("{}...", text.chars().take(117).collect::<String>())
                        } else {
                            text.clone()
                        };
                        first_user_message = Some(preview);
                        break;
                    }
                }
            }
            for block in &turn.blocks {
                if let ContentBlock::ToolUse { tool_use } = block {
                    tools_used.push(tool_use.name.clone());
                }
            }
        }

        // Dedupe tools while preserving order
        let mut seen = std::collections::HashSet::new();
        let tools_used: Vec<String> = tools_used
            .into_iter()
            .filter(|n| seen.insert(n.clone()))
            .collect();

        SessionSummary {
            session_id: self.id.0.clone(),
            status: self.status.clone(),
            run_duration_ms: self.last_run_duration_ms,
            total_prompt_tokens: self.total_prompt_tokens,
            total_completion_tokens: self.total_completion_tokens,
            turn_count: self.turn_count(),
            tools_used,
            first_user_message,
            created_at: self.created_at,
        }
    }
}

/// Summary generated at session end for display or logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub status: SessionStatus,
    pub run_duration_ms: Option<u64>,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub turn_count: usize,
    pub tools_used: Vec<String>,
    pub first_user_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl SessionSummary {
    /// Total tokens (prompt + completion).
    pub fn total_tokens(&self) -> u64 {
        self.total_prompt_tokens.saturating_add(self.total_completion_tokens)
    }

    /// Format run duration as "X.XXXs" or "—" if unknown.
    pub fn run_duration_display(&self) -> String {
        match self.run_duration_ms {
            Some(ms) => {
                let secs = ms / 1000;
                let rem = ms % 1000;
                format!("{}.{:03}s", secs, rem)
            }
            None => "—".to_string(),
        }
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
            SessionStatus::Cancelled,
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

    #[test]
    fn test_session_run_duration() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let mut session = Session::new(PathBuf::from("/repo"), config);

        assert!(session.last_run_duration_ms().is_none());
        session.finish_run(Some(5000));
        assert_eq!(session.last_run_duration_ms(), Some(5000));

        session.start_run();
        assert!(session.run_started_at.is_some());
        session.finish_run(None);
        assert!(session.run_started_at.is_none());
        assert!(session.last_run_duration_ms().is_some());
    }

    #[test]
    fn test_session_new_continuing() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let prev = Session::new(PathBuf::from("/repo"), config.clone());
        let prev_id = prev.id.0.clone();

        let next = Session::new_continuing(&prev);
        assert_ne!(next.id.0, prev_id);
        assert_eq!(next.repo_root, prev.repo_root);
        assert_eq!(next.config.model, prev.config.model);
        assert!(next.turns.is_empty());
        assert_eq!(next.parent_session_id.as_ref().map(|p| p.as_str()), Some(prev_id.as_str()));
    }

    #[test]
    fn test_session_token_totals() {
        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let mut session = Session::new(PathBuf::from("/repo"), config);

        assert_eq!(session.total_tokens(), 0);
        session.add_llm_usage(100, 50);
        assert_eq!(session.total_prompt_tokens, 100);
        assert_eq!(session.total_completion_tokens, 50);
        assert_eq!(session.total_tokens(), 150);
        session.add_llm_usage(200, 80);
        assert_eq!(session.total_prompt_tokens, 300);
        assert_eq!(session.total_completion_tokens, 130);
        assert_eq!(session.total_tokens(), 430);
    }

    #[test]
    fn test_session_build_summary() {
        use crate::turn::ContentBlock;
        use crate::tool_call::ToolUse;

        let config = SessionConfig::new("claude-sonnet-4", "anthropic");
        let mut session = Session::new(PathBuf::from("/repo"), config);
        session.add_llm_usage(50, 25);
        session.finish_run(Some(5000));
        session.set_status(SessionStatus::Completed);
        session.add_turn(Turn::user().with_block(ContentBlock::text("Hello, fix the bug")));
        session.add_turn(
            Turn::assistant()
                .with_block(ContentBlock::text("I'll help."))
                .with_block(ContentBlock::tool_use(ToolUse::new("t1", "bash", serde_json::json!({"command": "ls"})))),
        );

        let summary = session.build_summary();
        assert_eq!(summary.session_id, session.id.0);
        assert_eq!(summary.run_duration_ms, Some(5000));
        assert_eq!(summary.total_prompt_tokens, 50);
        assert_eq!(summary.total_completion_tokens, 25);
        assert_eq!(summary.turn_count, 2);
        assert_eq!(summary.tools_used, &["bash"]);
        assert_eq!(summary.first_user_message.as_deref(), Some("Hello, fix the bug"));
        assert_eq!(summary.run_duration_display(), "5.000s");
    }
}
