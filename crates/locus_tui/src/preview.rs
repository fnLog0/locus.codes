//! Seeded preview state for reviewing the TUI without a runtime.

use std::time::{Duration, Instant};

use crate::messages::{
    memory::MemoryMessage,
    meta_tool::{MetaToolKind, MetaToolMessage},
    tool::{EditDiffMessage, ToolCallMessage},
};
use crate::state::{Screen, TuiState};
use crate::theme::Appearance;
use crate::web_automation::state::{AutomationStatus, WebAutomationState};

fn sample_old_file() -> String {
    [
        "fn render_preview() {",
        "    let theme = load_theme();",
        "    let tools = collect_tools();",
        "    let mut rows = Vec::new();",
        "    rows.push(header(theme));",
        "    rows.push(summary());",
        "    rows.push(section(\"chat\"));",
        "    rows.push(section(\"tools\"));",
        "    rows.push(section(\"memory\"));",
        "    rows.push(section(\"errors\"));",
        "    rows.push(section(\"footer\"));",
        "    rows.push(section(\"shortcuts\"));",
        "    rows.push(section(\"scrollbar\"));",
        "    rows.push(section(\"empty-state\"));",
        "    rows.push(section(\"web\"));",
        "    finalize(rows);",
        "}",
    ]
    .join("\n")
}

fn sample_new_file() -> String {
    [
        "fn render_preview() {",
        "    let theme = load_theme();",
        "    let tools = collect_tools();",
        "    let mut rows = Vec::new();",
        "    rows.push(header(theme));",
        "    rows.push(summary());",
        "    rows.push(section(\"chat\"));",
        "    rows.push(section(\"tools\"));",
        "    rows.push(section(\"memory\"));",
        "    rows.push(section(\"errors\"));",
        "    rows.push(section(\"footer\"));",
        "    rows.push(section(\"shortcuts\"));",
        "    rows.push(section(\"scrollbar\"));",
        "    rows.push(section(\"empty-state\"));",
        "    rows.push(section(\"web-automation\"));",
        "    rows.push(section(\"preview-mode\"));",
        "    finalize(rows);",
        "}",
    ]
    .join("\n")
}

fn preview_web_automation() -> WebAutomationState {
    WebAutomationState {
        status: AutomationStatus::Completed,
        url: "https://example.com/dashboard".to_string(),
        goal: "Capture the page title, primary CTA, and any blocking error banners.".to_string(),
        run_id: Some("preview-run-001".to_string()),
        streaming_url: Some("http://127.0.0.1:9222/devtools/preview-run-001".to_string()),
        progress_messages: vec![
            "Started: run_id=preview-run-001".to_string(),
            "Open landing page".to_string(),
            "Wait for dashboard shell".to_string(),
            "Extract title and primary action".to_string(),
            "Completed: success".to_string(),
        ],
        result: Some(serde_json::json!({
            "title": "Example Dashboard",
            "primary_cta": "Create report",
            "errors": [],
        })),
        error: None,
        started_at: Some(Instant::now() - Duration::from_secs(83)),
        duration_ms: Some(3_240),
        scroll: 0,
    }
}

/// Build a seeded [TuiState] for `locus tui --preview`.
pub fn preview_state(show_onboarding: bool, appearance: Appearance) -> TuiState {
    let mut state = TuiState::with_appearance(appearance);
    if show_onboarding {
        state.screen = Screen::Onboarding;
    }

    state.status = "Preview mode · mock data loaded".to_string();
    state.status_permanent = true;
    state.status_set_at = None;

    state.push_trace_line("[preview] TUI preview loaded".to_string());
    state.push_trace_line("[preview] Press Ctrl+D for logs, Ctrl+W for web automation".to_string());
    state.push_trace_line("[preview] This mode runs without the runtime or network calls".to_string());
    for idx in 1..=8 {
        state.push_trace_line(format!(
            "[preview] sample trace {} · rendering subsystem healthy",
            idx
        ));
    }

    state.push_separator("Preview session".to_string());
    state.push_user(
        "Show me the full locus.codes TUI surface with mock data so I can review the layout."
            .to_string(),
        Some("09:41".to_string()),
    );
    state.push_ai(
        "This preview includes transcript hierarchy, grouped tools, diffs, memory events, errors, logs, web automation, and a wrapped input draft."
            .to_string(),
        Some("09:41".to_string()),
    );
    state.push_think(
        "Collecting representative blocks for chat, execution logs, memory, and failures.\nChecking dense transcript spacing.\nPreparing diff preview and footer draft."
            .to_string(),
        true,
    );
    state.push_meta_tool(MetaToolMessage::done(
        MetaToolKind::ToolSearch,
        84,
        true,
        Some("find layout and rendering modules".to_string()),
    ));
    state.push_meta_tool(MetaToolMessage::error(
        MetaToolKind::Task,
        "sub-agent quota exceeded in preview sample",
        Some("audit chrome consistency".to_string()),
    ));

    state.push_tool_grouped(ToolCallMessage::done(
        Some("preview-tool-grep".to_string()),
        "grep",
        58,
        true,
        Some("crates/locus_tui/src/view.rs".to_string()),
        None,
    ));
    state.push_tool_grouped(ToolCallMessage::done(
        Some("preview-tool-edit".to_string()),
        "edit_file",
        182,
        true,
        Some("crates/locus_tui/src/view.rs".to_string()),
        None,
    ));
    state.push_tool_grouped(ToolCallMessage::error(
        Some("preview-tool-test".to_string()),
        "cargo test",
        "snapshot mismatch in preview fixture",
        Some("-p locus-tui".to_string()),
    ));
    state.insert_edit_diff_after_tool(
        "preview-tool-edit",
        EditDiffMessage {
            path: "crates/locus_tui/src/view.rs".to_string(),
            old_content: sample_old_file(),
            new_content: sample_new_file(),
            tool_id: Some("preview-tool-edit".to_string()),
        },
    );

    state.push_memory(MemoryMessage::recall(
        "tui design direction and transcript spacing",
        4,
    ));
    state.push_memory(MemoryMessage::store(
        "project:locuscodes_preview",
        "decision",
        "Preview mode should launch with seeded chat data and no runtime dependencies.",
    ));
    state.push_error(
        "Preview runtime error sample: LocusGraph transport failed while refreshing memory context."
            .to_string(),
        Some("09:42".to_string()),
    );

    state.push_separator("Follow-up".to_string());
    state.push_user(
        "Can I inspect secondary screens too?".to_string(),
        Some("09:43".to_string()),
    );
    state.push_ai(
        "Yes. Use Ctrl+D for runtime logs, Ctrl+W for web automation, and --onboarding if you want the setup screen first."
            .to_string(),
        Some("09:43".to_string()),
    );

    state.input_buffer =
        "Draft a follow-up patch that adds a polished preview mode and keeps the transcript readable on small terminals."
            .to_string();
    state.input_cursor = state.input_buffer.len();
    state.web_automation = preview_web_automation();
    state.cache_dirty = true;
    state.needs_redraw = true;
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ChatItem;

    #[test]
    fn preview_state_seeds_messages_and_input() {
        let state = preview_state(false, Appearance::Dark);
        assert!(!state.messages.is_empty());
        assert!(!state.trace_lines.is_empty());
        assert!(!state.input_buffer.is_empty());
        assert!(matches!(state.messages[0], ChatItem::Separator(_)));
    }

    #[test]
    fn preview_state_can_start_on_onboarding() {
        let state = preview_state(true, Appearance::Light);
        assert_eq!(state.screen, Screen::Onboarding);
        assert!(!state.messages.is_empty());
    }
}
