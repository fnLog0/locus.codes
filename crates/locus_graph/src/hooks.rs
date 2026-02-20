//! Pre-built hooks for storing events at specific points in the agent loop.
//!
//! Each hook builds a `CreateEventRequest` and fires it async (non-blocking).

use crate::client::LocusGraphClient;
use crate::types::{CreateEventRequest, EventKind};
use serde_json::json;

// Context ID constants
pub const CONTEXT_TERMINAL: &str = "terminal";
pub const CONTEXT_EDITOR: &str = "editor";
pub const CONTEXT_USER_INTENT: &str = "user_intent";
pub const CONTEXT_ERRORS: &str = "errors";
pub const CONTEXT_DECISIONS: &str = "decisions";

impl LocusGraphClient {
    /// After executing any tool (bash, grep, edit_file, etc.)
    ///
    /// Context ID: `terminal`
    pub async fn store_tool_run(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        result: &serde_json::Value,
        duration_ms: u64,
        is_error: bool,
    ) {
        let event = CreateEventRequest::new(
            if is_error {
                EventKind::Observation
            } else {
                EventKind::Action
            },
            json!({
                "kind": "tool_run",
                "data": {
                    "tool": tool_name,
                    "args": args,
                    "result_preview": truncate_result(result),
                    "duration_ms": duration_ms,
                    "is_error": is_error,
                }
            }),
        )
        .context_id(format!("{}:{}", CONTEXT_TERMINAL, tool_name))
        .source("agent");

        self.store_event(event).await;
    }

    /// After writing/editing a file
    ///
    /// Context ID: `editor`
    pub async fn store_file_edit(
        &self,
        path: &str,
        summary: &str,
        diff_preview: Option<&str>,
    ) {
        let event = CreateEventRequest::new(
            EventKind::Action,
            json!({
                "kind": "file_edit",
                "data": {
                    "path": path,
                    "summary": summary,
                    "diff_preview": diff_preview,
                }
            }),
        )
        .context_id(format!("{}:{}", CONTEXT_EDITOR, path_to_context(path)))
        .source("agent");

        self.store_event(event).await;
    }

    /// When user sends a message
    ///
    /// Context ID: `user_intent`
    pub async fn store_user_intent(&self, message: &str, intent_summary: &str) {
        let event = CreateEventRequest::new(
            EventKind::Observation,
            json!({
                "kind": "user_intent",
                "data": {
                    "message_preview": truncate_string(message, 500),
                    "intent_summary": intent_summary,
                }
            }),
        )
        .context_id(CONTEXT_USER_INTENT)
        .source("user");

        self.store_event(event).await;
    }

    /// On any error (tool failure, LLM error, etc.)
    ///
    /// Context ID: `errors`
    pub async fn store_error(
        &self,
        context: &str,
        error_message: &str,
        command_or_file: Option<&str>,
    ) {
        let event = CreateEventRequest::new(
            EventKind::Observation,
            json!({
                "kind": "error",
                "data": {
                    "context": context,
                    "error_message": error_message,
                    "command_or_file": command_or_file,
                }
            }),
        )
        .context_id(CONTEXT_ERRORS)
        .source("system");

        self.store_event(event).await;
    }

    /// After LLM responds — store the decision/reasoning
    ///
    /// Context ID: `decisions`
    pub async fn store_decision(&self, summary: &str, reasoning: Option<&str>) {
        let event = CreateEventRequest::new(
            EventKind::Decision,
            json!({
                "kind": "decision",
                "data": {
                    "summary": summary,
                    "reasoning": reasoning,
                }
            }),
        )
        .context_id(CONTEXT_DECISIONS)
        .source("agent");

        self.store_event(event).await;
    }

    /// When agent discovers project conventions
    ///
    /// Context ID: `project:{repo_hash}`
    pub async fn store_project_convention(
        &self,
        repo: &str,
        convention: &str,
        examples: Vec<&str>,
    ) {
        let event = CreateEventRequest::new(
            EventKind::Fact,
            json!({
                "kind": "project_convention",
                "data": {
                    "repo": repo,
                    "convention": convention,
                    "examples": examples,
                }
            }),
        )
        .context_id(format!("project:{}", simple_hash(repo)))
        .source("agent");

        self.store_event(event).await;
    }

    /// When a pattern is validated (becomes a learned skill)
    ///
    /// Context ID: `skill:{name}`
    pub async fn store_skill(
        &self,
        name: &str,
        description: &str,
        steps: Vec<&str>,
        validated: bool,
    ) {
        let event = CreateEventRequest::new(
            EventKind::Fact,
            json!({
                "kind": "skill",
                "data": {
                    "name": name,
                    "description": description,
                    "steps": steps,
                    "validated": validated,
                }
            }),
        )
        .context_id(format!("skill:{}", name))
        .source("agent");

        self.store_event(event).await;
    }
}

/// Truncate a result value for storage.
fn truncate_result(value: &serde_json::Value) -> serde_json::Value {
    let s = serde_json::to_string(value).unwrap_or_default();
    if s.len() > 1000 {
        json!({
            "truncated": true,
            "preview": &s[..500],
            "length": s.len(),
        })
    } else {
        value.clone()
    }
}

/// Truncate a string to a maximum length.
fn truncate_string(s: &str, max_len: usize) -> &str {
    if s.len() > max_len {
        &s[..max_len]
    } else {
        s
    }
}

/// Convert a file path to a context-safe string.
fn path_to_context(path: &str) -> String {
    path.replace(['/', '\\', '.', ':'], "_")
}

/// Simple hash function for repo names.
fn simple_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
