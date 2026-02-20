//! Pre-built hooks for storing events at specific points in the agent loop.
//!
//! Each hook builds a `CreateEventRequest` and fires it async (non-blocking).
//!
//! ## Source Priority (high → low)
//!
//! | Source       | Confidence | Use Case                          |
//! |--------------|------------|-----------------------------------|
//! | `validator`  | 0.9        | Authoritative, runtime-verified   |
//! | `executor`   | 0.8        | Reliable, task completion         |
//! | `user`       | 0.7        | Valuable but subjective           |
//! | `agent`      | 0.6        | Agent decisions                   |
//! | `system`     | 0.5        | System events (default)           |
//!
//! ## Context IDs
//!
//! **Constants** (one per feedback loop):
//! | Constant            | Use Case                  |
//! |---------------------|---------------------------|
//! | `terminal`          | Tool execution results    |
//! | `editor`            | File modifications        |
//! | `user_intent`       | User messages             |
//! | `errors`            | Error tracking            |
//! | `decisions`         | Agent reasoning           |
//!
//! **Dynamic patterns**:
//! | Pattern             | Use Case                  |
//! |---------------------|---------------------------|
//! | `project:{hash}`    | Repo conventions          |
//! | `skill:{name}`      | Learned patterns          |
//! | `llm:{model}`       | LLM usage tracking        |
//! | `test:{file}`       | Test results              |
//! | `git:{hash}`        | VCS operations            |

use crate::client::LocusGraphClient;
use crate::types::{CreateEventRequest, EventKind};
use serde_json::json;

/// Context ID constants (one per feedback loop)
pub const CONTEXT_DECISIONS: &str = "decisions";
pub const CONTEXT_EDITOR: &str = "editor";
pub const CONTEXT_ERRORS: &str = "errors";
pub const CONTEXT_TERMINAL: &str = "terminal";
pub const CONTEXT_TOOLS: &str = "tools";
pub const CONTEXT_USER_INTENT: &str = "user_intent";

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
        .source("executor");

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
        .source("executor");

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

    /// When LLM is called — track model usage and tokens
    ///
    /// Context ID: `llm:{model}`
    pub async fn store_llm_call(
        &self,
        model: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
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
                "kind": "llm_call",
                "data": {
                    "model": model,
                    "prompt_tokens": prompt_tokens,
                    "completion_tokens": completion_tokens,
                    "total_tokens": prompt_tokens + completion_tokens,
                    "duration_ms": duration_ms,
                    "is_error": is_error,
                }
            }),
        )
        .context_id(format!("llm:{}", model.replace(['/', '.', ':'], "_")))
        .source("executor");

        self.store_event(event).await;
    }

    /// After running tests — track pass/fail and duration
    ///
    /// Context ID: `test:{test_file}`
    pub async fn store_test_run(
        &self,
        test_file: &str,
        passed: u32,
        failed: u32,
        duration_ms: u64,
        output_preview: Option<&str>,
    ) {
        let event = CreateEventRequest::new(
            if failed > 0 {
                EventKind::Observation
            } else {
                EventKind::Action
            },
            json!({
                "kind": "test_run",
                "data": {
                    "test_file": test_file,
                    "passed": passed,
                    "failed": failed,
                    "total": passed + failed,
                    "duration_ms": duration_ms,
                    "output_preview": output_preview.map(|s| truncate_string(s, 500)),
                }
            }),
        )
        .context_id(format!("test:{}", path_to_context(test_file)))
        .source("executor");

        self.store_event(event).await;
    }

    /// After git operations — track version control actions
    ///
    /// Context ID: `git:{repo_hash}`
    pub async fn store_git_op(
        &self,
        repo: &str,
        operation: &str,
        details: Option<&str>,
        is_error: bool,
    ) {
        let event = CreateEventRequest::new(
            if is_error {
                EventKind::Observation
            } else {
                EventKind::Action
            },
            json!({
                "kind": "git_op",
                "data": {
                    "repo": repo,
                    "operation": operation,
                    "details": details,
                    "is_error": is_error,
                }
            }),
        )
        .context_id(format!("git:{}", simple_hash(repo)))
        .source("executor");

        self.store_event(event).await;
    }

    /// Register a tool schema as a memory.
    ///
    /// Called at startup for ToolBus tools, and on connect for MCP/ACP tools.
    /// Stores the tool's description and schema so `retrieve_memories()` can
    /// surface it when the user's intent matches.
    ///
    /// Context ID: `tool:{tool_name}`
    pub async fn store_tool_schema(
        &self,
        tool_name: &str,
        description: &str,
        parameters_schema: &serde_json::Value,
        source_type: &str, // "toolbus", "mcp", "acp"
        tags: Vec<&str>,
    ) {
        let event = CreateEventRequest::new(
            EventKind::Fact,
            json!({
                "kind": "tool_schema",
                "data": {
                    "tool": tool_name,
                    "description": description,
                    "parameters": parameters_schema,
                    "source_type": source_type,
                    "tags": tags,
                }
            }),
        )
        .context_id(format!("tool:{}", tool_name))
        .related_to(vec![CONTEXT_TOOLS.to_string()])
        .source("system");

        self.store_event(event).await;
    }

    /// Store a tool usage pattern for discovery learning.
    ///
    /// Called after a tool is successfully used. Links user intent to tool
    /// so future `retrieve_memories()` calls surface this tool for similar intents.
    ///
    /// Context ID: `tool:{tool_name}:usage`
    pub async fn store_tool_usage(
        &self,
        tool_name: &str,
        user_intent: &str,
        success: bool,
        duration_ms: u64,
    ) {
        let event = CreateEventRequest::new(
            if success {
                EventKind::Action
            } else {
                EventKind::Observation
            },
            json!({
                "kind": "tool_usage",
                "data": {
                    "tool": tool_name,
                    "intent": user_intent,
                    "success": success,
                    "duration_ms": duration_ms,
                }
            }),
        )
        .context_id(format!("tool:{}:usage", tool_name))
        .related_to(vec![format!("tool:{}", tool_name)])
        .source("executor");

        self.store_event(event).await;
    }
}

/// Truncate a result value for storage (avoids double serialization).
fn truncate_result(value: &serde_json::Value) -> serde_json::Value {
    match serde_json::to_string(value) {
        Ok(s) if s.len() > 1000 => {
            // Find char boundary at or before 500 chars to avoid slicing mid-UTF-8
            let end = s
                .char_indices()
                .take_while(|(i, _)| *i < 500)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
            json!({
                "truncated": true,
                "preview": &s[..end],
                "length": s.len(),
            })
        }
        Ok(_) => value.clone(),
        Err(_) => json!({ "error": "serialization_failed" }),
    }
}

/// Truncate a string to a maximum length (UTF-8 safe).
fn truncate_string(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }
    // Find the last char boundary at or before max_len
    let mut end = max_len;
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    &s[..end]
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
