//! Pre-built hooks for storing events at specific points in the agent loop.
//!
//! Each hook builds a `CreateEventRequest` and fires it async (non-blocking).
//! Hooks automatically set semantic links (related_to, extends, reinforces,
//! contradicts) based on event semantics, and merge with caller-provided links.
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
//! Backend requires format `type:name` (e.g. fact:redis_caching). We use the `fact` type throughout.
//!
//! **Constants** (for links; type aligned with event_kind):
//! | Constant              | Value                   | Use Case          |
//! | CONTEXT_USER_INTENT   | observation:user_intent | User messages     |
//! | CONTEXT_DECISIONS     | decision:decisions      | Agent reasoning   |
//! | CONTEXT_ERRORS       | observation:errors      | Error tracking    |
//! | CONTEXT_TOOLS        | fact:tools              | Tool schemas      |
//! | CONTEXT_EDITOR_LINK   | action:editor           | File/test/git     |
//!
//! **Dynamic context_id** (type matches event_kind): action:*, observation:*, decision:*, fact:*

use crate::client::LocusGraphClient;
use crate::types::{CreateEventRequest, EventKind, EventLinks};
use serde_json::json;

/// Context ID constants (one per feedback loop).
/// Backend requires format `type:name` (e.g. fact:redis_caching). Type is aligned with event_kind.
pub const CONTEXT_DECISIONS: &str = "decision:decisions";
pub const CONTEXT_EDITOR: &str = "editor";
/// Use in related_to/contradicts/reinforces.
pub const CONTEXT_EDITOR_LINK: &str = "action:editor";
pub const CONTEXT_ERRORS: &str = "observation:errors";
pub const CONTEXT_TERMINAL: &str = "terminal";
pub const CONTEXT_TOOLS: &str = "fact:tools";
pub const CONTEXT_USER_INTENT: &str = "observation:user_intent";

/// Apply links to a CreateEventRequest, merging with any existing links.
fn apply_links(mut event: CreateEventRequest, links: EventLinks) -> CreateEventRequest {
    if !links.related_to.is_empty() {
        let mut existing = event.related_to.unwrap_or_default();
        existing.extend(links.related_to);
        event.related_to = Some(existing);
    }
    if !links.extends.is_empty() {
        let mut existing = event.extends.unwrap_or_default();
        existing.extend(links.extends);
        event.extends = Some(existing);
    }
    if !links.reinforces.is_empty() {
        let mut existing = event.reinforces.unwrap_or_default();
        existing.extend(links.reinforces);
        event.reinforces = Some(existing);
    }
    if !links.contradicts.is_empty() {
        let mut existing = event.contradicts.unwrap_or_default();
        existing.extend(links.contradicts);
        event.contradicts = Some(existing);
    }
    event
}

impl LocusGraphClient {
    /// After executing any tool (bash, grep, edit_file, etc.)
    ///
    /// Auto-links: `related_to: [fact:user_intent]`. Context ID: `fact:terminal_{tool_name}`
    pub async fn store_tool_run(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        result: &serde_json::Value,
        duration_ms: u64,
        is_error: bool,
        links: EventLinks,
    ) {
        let auto_links = EventLinks::new()
            .related_to(CONTEXT_USER_INTENT);

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
        .context_id(format!("action:terminal_{}", safe_context_name(tool_name)))
        .source("executor");

        self.store_event(apply_links(event, auto_links.merge(links))).await;
    }

    /// After writing/editing a file
    ///
    /// Auto-links: `related_to: [fact:decisions]`. Context ID: `fact:editor_{path}`
    pub async fn store_file_edit(
        &self,
        path: &str,
        summary: &str,
        diff_preview: Option<&str>,
        links: EventLinks,
    ) {
        let auto_links = EventLinks::new()
            .related_to(CONTEXT_DECISIONS);

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
        .context_id(format!("action:editor_{}", safe_context_name(&path_to_context(path))))
        .source("executor");

        self.store_event(apply_links(event, auto_links.merge(links))).await;
    }

    /// When user sends a message
    ///
    /// No auto-links — user intent is a root event. Context ID: `fact:user_intent`
    pub async fn store_user_intent(
        &self,
        message: &str,
        intent_summary: &str,
        links: EventLinks,
    ) {
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
        .context_id("observation:user_intent")
        .source("user");

        self.store_event(apply_links(event, links)).await;
    }

    /// On any error (tool failure, LLM error, etc.)
    ///
    /// No auto-links — caller should provide `contradicts` (use fact:terminal_{tool} etc.).
    /// Context ID: `fact:errors`
    pub async fn store_error(
        &self,
        context: &str,
        error_message: &str,
        command_or_file: Option<&str>,
        links: EventLinks,
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
        .context_id("observation:errors")
        .source("system");

        self.store_event(apply_links(event, links)).await;
    }

    /// After LLM responds — store the decision/reasoning
    ///
    /// Auto-links: `extends: [fact:user_intent]`. Context ID: `fact:decisions`
    pub async fn store_decision(
        &self,
        summary: &str,
        reasoning: Option<&str>,
        links: EventLinks,
    ) {
        let auto_links = EventLinks::new()
            .extends(CONTEXT_USER_INTENT);

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
        .context_id("decision:decisions")
        .source("agent");

        self.store_event(apply_links(event, auto_links.merge(links))).await;
    }

    /// When agent discovers project conventions
    ///
    /// No auto-links — caller should provide `reinforces` for existing conventions
    /// or `contradicts` if this replaces an old one. Context ID: `fact:project_{hash}`
    pub async fn store_project_convention(
        &self,
        repo: &str,
        convention: &str,
        examples: Vec<&str>,
        links: EventLinks,
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
        .context_id(format!("fact:project_{}", simple_hash(repo)))
        .source("agent");

        self.store_event(apply_links(event, links)).await;
    }

    /// When a pattern is validated (becomes a learned skill)
    ///
    /// No auto-links — caller should provide `reinforces` for prior observations
    /// or `contradicts` for superseded skills. Context ID: `fact:skill_{name}`
    pub async fn store_skill(
        &self,
        name: &str,
        description: &str,
        steps: Vec<&str>,
        validated: bool,
        links: EventLinks,
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
        .context_id(format!("fact:skill_{}", safe_context_name(name)))
        .source("agent");

        self.store_event(apply_links(event, links)).await;
    }

    /// When LLM is called — track model usage and tokens
    ///
    /// Auto-links: `related_to: [fact:decisions]`. Context ID: `fact:llm_{model}`
    pub async fn store_llm_call(
        &self,
        model: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
        duration_ms: u64,
        is_error: bool,
        links: EventLinks,
    ) {
        let auto_links = EventLinks::new()
            .related_to(CONTEXT_DECISIONS);

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
        .context_id(format!("action:llm_{}", safe_context_name(&model.replace(['/', '.', ':'], "_"))))
        .source("executor");

        self.store_event(apply_links(event, auto_links.merge(links))).await;
    }

    /// After running tests — track pass/fail and duration
    ///
    /// Auto-links: if tests pass → `reinforces: [fact:editor]`, if fail → `contradicts: [fact:editor]`.
    /// Context ID: `fact:test_{path}`
    pub async fn store_test_run(
        &self,
        test_file: &str,
        passed: u32,
        failed: u32,
        duration_ms: u64,
        output_preview: Option<&str>,
        links: EventLinks,
    ) {
        let auto_links = if failed > 0 {
            EventLinks::new().contradicts(CONTEXT_EDITOR_LINK)
        } else {
            EventLinks::new().reinforces(CONTEXT_EDITOR_LINK)
        };

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
        .context_id(format!("action:test_{}", safe_context_name(&path_to_context(test_file))))
        .source("executor");

        self.store_event(apply_links(event, auto_links.merge(links))).await;
    }

    /// After git operations — track version control actions
    ///
    /// Auto-links: `related_to: [fact:editor]`. Context ID: `fact:git_{hash}`
    pub async fn store_git_op(
        &self,
        repo: &str,
        operation: &str,
        details: Option<&str>,
        is_error: bool,
        links: EventLinks,
    ) {
        let auto_links = EventLinks::new()
            .related_to(CONTEXT_EDITOR_LINK);

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
        .context_id(format!("action:git_{}", simple_hash(repo)))
        .source("executor");

        self.store_event(apply_links(event, auto_links.merge(links))).await;
    }

    /// Register a tool schema as a memory.
    ///
    /// Called at startup for ToolBus tools, and on connect for MCP/ACP tools.
    /// Stores the tool's description and schema so `retrieve_memories()` can
    /// surface it when the user's intent matches.
    ///
    /// Auto-links: `related_to: [fact:tools]`. Context ID: `fact:tool_{tool_name}`
    pub async fn store_tool_schema(
        &self,
        tool_name: &str,
        description: &str,
        parameters_schema: &serde_json::Value,
        source_type: &str, // "toolbus", "mcp", "acp"
        tags: Vec<&str>,
        links: EventLinks,
    ) {
        let auto_links = EventLinks::new()
            .related_to(CONTEXT_TOOLS);

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
        .context_id(format!("fact:tool_{}", safe_context_name(tool_name)))
        .source("system");

        self.store_event(apply_links(event, auto_links.merge(links))).await;
    }

    /// Store a tool usage pattern for discovery learning.
    ///
    /// Called after a tool is successfully used. Links user intent to tool
    /// so future `retrieve_memories()` calls surface this tool for similar intents.
    ///
    /// Auto-links: `related_to: [fact:tool_{tool_name}]`.
    /// Context ID: `fact:tool_usage_{tool_name}`
    pub async fn store_tool_usage(
        &self,
        tool_name: &str,
        user_intent: &str,
        success: bool,
        duration_ms: u64,
        links: EventLinks,
    ) {
        let tool_ctx = format!("fact:tool_{}", safe_context_name(tool_name));
        let auto_links = EventLinks::new().related_to(tool_ctx.clone());

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
        .context_id(format!("action:tool_usage_{}", safe_context_name(tool_name)))
        .source("executor");

        self.store_event(apply_links(event, auto_links.merge(links))).await;
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

/// Sanitize a string for use in context_id name part (backend expects type:name, name = [a-z0-9_]).
fn safe_context_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

/// Simple hash function for repo names.
fn simple_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
