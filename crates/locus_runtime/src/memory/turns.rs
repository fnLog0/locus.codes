use chrono::Utc;
use locus_graph::{CreateEventRequest, EventKind, TurnSummary};
use serde_json::json;

/// Build a turn start event (anchor). The payload is lightweight because
/// the summary is written at turn end.
pub fn build_turn_start(
    turn_ctx: &str,
    session_ctx: &str,
    user_message: &str,
    turn_sequence: u32,
) -> CreateEventRequest {
    CreateEventRequest::new(
        EventKind::Action,
        json!({
            "kind": "turn",
            "data": {
                "status": "active",
                "turn_sequence": turn_sequence,
                "user_message": truncate(user_message, 500),
                "started_at": Utc::now().to_rfc3339(),
            }
        }),
    )
    .context_id(turn_ctx)
    .extends(vec![session_ctx.to_string()])
    .source("executor")
}

/// Build a turn end event that overwrites the anchor with the summary.
pub fn build_turn_end(
    turn_ctx: &str,
    session_ctx: &str,
    summary: TurnSummary,
    turn_sequence: u32,
    duration_ms: u64,
) -> CreateEventRequest {
    let TurnSummary {
        title,
        user_request,
        actions_taken,
        outcome,
        decisions,
        files_read,
        files_modified,
        event_count,
    } = summary;

    CreateEventRequest::new(
        EventKind::Action,
        json!({
            "kind": "turn",
            "data": {
                "status": "completed",
                "turn_sequence": turn_sequence,
                "title": title,
                "user_request": user_request,
                "actions_taken": actions_taken,
                "outcome": outcome,
                "decisions": decisions,
                "files_read": files_read,
                "files_modified": files_modified,
                "event_count": event_count,
                "duration_ms": duration_ms,
                "ended_at": Utc::now().to_rfc3339(),
            }
        }),
    )
    .context_id(turn_ctx)
    .extends(vec![session_ctx.to_string()])
    .source("executor")
}

/// Build an action event that captures tool calls.
pub fn build_action_event(
    event_ctx: &str,
    turn_ctx: &str,
    tool_name: &str,
    tool_args: &serde_json::Value,
    result: &serde_json::Value,
    is_error: bool,
    duration_ms: u64,
) -> CreateEventRequest {
    CreateEventRequest::new(
        EventKind::Action,
        json!({
            "kind": "tool_call",
            "data": {
                "tool": tool_name,
                "args_summary": truncate(&tool_args.to_string(), 300),
                "result_summary": truncate(&result.to_string(), 500),
                "is_error": is_error,
                "duration_ms": duration_ms,
            }
        }),
    )
    .context_id(event_ctx)
    .extends(vec![turn_ctx.to_string()])
    .source("executor")
}

/// Build an LLM call event with token counts and duration.
pub fn build_llm_event(
    event_ctx: &str,
    turn_ctx: &str,
    model: &str,
    prompt_tokens: u64,
    completion_tokens: u64,
    duration_ms: u64,
    has_tool_calls: bool,
) -> CreateEventRequest {
    CreateEventRequest::new(
        EventKind::Observation,
        json!({
            "kind": "llm_call",
            "data": {
                "model": model,
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
                "total_tokens": prompt_tokens + completion_tokens,
                "duration_ms": duration_ms,
                "has_tool_calls": has_tool_calls,
            }
        }),
    )
    .context_id(event_ctx)
    .extends(vec![turn_ctx.to_string()])
    .source("executor")
}

/// Build an intent event capturing the user request for the turn.
pub fn build_intent_event(
    event_ctx: &str,
    turn_ctx: &str,
    user_message: &str,
) -> CreateEventRequest {
    CreateEventRequest::new(
        EventKind::Fact,
        json!({
            "kind": "intent",
            "data": {
                "message": truncate(user_message, 1000),
            }
        }),
    )
    .context_id(event_ctx)
    .extends(vec![turn_ctx.to_string()])
    .source("agent")
}

/// Build an error event tagging either tools or the LLM.
pub fn build_error_event(
    event_ctx: &str,
    turn_ctx: &str,
    error_source: &str,
    error_message: &str,
) -> CreateEventRequest {
    CreateEventRequest::new(
        EventKind::Observation,
        json!({
            "kind": "error",
            "data": {
                "source": error_source,
                "message": truncate(error_message, 500),
            }
        }),
    )
    .context_id(event_ctx)
    .extends(vec![turn_ctx.to_string()])
    .source("executor")
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s.chars().take(max_len).collect()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
