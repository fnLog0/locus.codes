//! Map [locus_core::SessionEvent] to [TuiState] updates.

use locus_core::{Role, SessionEvent, ToolUse};

use crate::messages::meta_tool::{MetaToolKind, MetaToolMessage};
use crate::messages::tool::ToolCallMessage;
use crate::state::{ChatItem, TuiState};

/// Apply a session event to TUI state (accumulate or push items).
/// Runtime logs are shown in the debug traces screen (Ctrl+D), not session events.
pub fn apply_session_event(state: &mut TuiState, event: SessionEvent) {
    state.needs_redraw = true;
    match event {
        SessionEvent::TurnStart { role } => {
            if role == Role::Assistant {
                state.current_ai_text.clear();
                state.current_think_text.clear();
                state.is_streaming = true;
            }
        }
        SessionEvent::TextDelta { text } => {
            state.current_ai_text.push_str(&text);
        }
        SessionEvent::ThinkingDelta { thinking } => {
            state.current_think_text.push_str(&thinking);
        }
        SessionEvent::ToolStart { tool_use } => {
            // Flush any accumulated thinking/AI text so it appears BEFORE the tool call.
            let think = std::mem::take(&mut state.current_think_text);
            if !think.is_empty() {
                state.push_think(think, false);
            }
            let ai = std::mem::take(&mut state.current_ai_text);
            if !ai.is_empty() {
                let ts = chrono::Local::now().format("%H:%M").to_string();
                state.push_ai(ai, Some(ts));
            }

            if let Some(kind) = MetaToolKind::from_name(&tool_use.name) {
                let detail = tool_detail(&tool_use);
                state.push_meta_tool(MetaToolMessage::running(kind, detail));
            } else {
                let summary = tool_summary(&tool_use);
                state.push_tool_grouped(ToolCallMessage::running(&tool_use.id, tool_use.name, summary));
            }
        }
        SessionEvent::ToolDone {
            tool_use_id,
            result,
        } => {
            state.cache_dirty = true;
            // Try to find and update the tool by its id
            if !state.update_tool_by_id(&tool_use_id, result.duration_ms, !result.is_error) {
                // Fallback: update last MetaTool if applicable
                if let Some(ChatItem::MetaTool(m)) = state.messages.last_mut() {
                    *m = MetaToolMessage::done(
                        m.kind,
                        result.duration_ms,
                        !result.is_error,
                        m.detail.clone(),
                    );
                    state.cache_dirty = true;
                }
            }
        }
        SessionEvent::Status { message } => {
            state.status = message;
            state.status_set_at = Some(std::time::Instant::now());
            state.status_permanent = false;
        }
        SessionEvent::TurnEnd => {
            state.is_streaming = false;
            state.flush_turn();
        }
        SessionEvent::Error { error } => {
            state.status = error.clone();
            state.status_set_at = Some(std::time::Instant::now());
            state.status_permanent = false;
            state.push_error(error, None);
        }
        SessionEvent::SessionEnd {
            prompt_tokens,
            completion_tokens,
            ..
        } => {
            state.is_streaming = false;
            state.flush_turn();
            let total = prompt_tokens + completion_tokens;
            let sep_label = if total > 0 {
                format!(
                    "Turn complete · {} tokens ({}↑ {}↓)",
                    format_token_count(total),
                    format_token_count(prompt_tokens),
                    format_token_count(completion_tokens),
                )
            } else {
                "Turn complete".to_string()
            };
            state.push_separator(sep_label);
            state.status = "Send message to continue · Ctrl+N new session".to_string();
            state.status_permanent = true;
            state.status_set_at = None;
        }
        SessionEvent::MemoryRecall { .. } => {}
    }
}

/// Format token count for display: "1,234" or "12.3k" for large numbers.
fn format_token_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else if n >= 1_000 {
        // Add comma separator: 1,234
        let s = n.to_string();
        let (head, tail) = s.split_at(s.len() - 3);
        format!("{},{}", head, tail)
    } else {
        n.to_string()
    }
}

fn tool_summary(tool: &ToolUse) -> Option<String> {
    tool.args
        .get("path")
        .or(tool.args.get("file_path"))
        .or(tool.args.get("file"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| tool.args.get("command").and_then(|v| v.as_str()).map(|s| s.to_string()))
}

fn tool_detail(tool: &ToolUse) -> Option<String> {
    tool.args
        .get("query")
        .or(tool.args.get("tool_id"))
        .or(tool.args.get("description"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
