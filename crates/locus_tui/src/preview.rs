//! Seeded preview state for reviewing the TUI without a runtime.

use std::time::{Duration, Instant};

use crate::messages::{
    memory::MemoryMessage,
    meta_tools::{MetaToolKind, MetaToolMessage},
    tools::{EditDiffMessage, ToolCallMessage},
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

/// Create a tool call message with args and result for per-tool rendering.
fn tool_with_data(
    id: &str,
    tool_name: &str,
    duration_ms: u64,
    success: bool,
    args: serde_json::Value,
    result: serde_json::Value,
) -> ToolCallMessage {
    let mut msg = ToolCallMessage::done(
        Some(id.to_string()),
        tool_name,
        duration_ms,
        success,
        None,
        None,
    );
    msg.args = Some(args);
    msg.result = Some(result);
    msg
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
    state.push_trace_line(
        "[preview] This mode runs without the runtime or network calls".to_string(),
    );
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
        "This preview includes all per-tool rendering: bash, edit_file, create_file, undo_edit, read, glob, grep, finder, handoff, task_list, and web automation."
            .to_string(),
        Some("09:41".to_string()),
    );
    state.push_think(
        "Collecting representative blocks for each tool type.\nChecking per-tool summaries and previews.\nPreparing diff preview and footer draft."
            .to_string(),
        true,
    );

    // BASH TOOL
    state.push_tool_grouped(tool_with_data(
        "tool-bash-1",
        "bash",
        245,
        true,
        serde_json::json!({ "command": "cargo build --release" }),
        serde_json::json!({
            "stdout": "Compiling locus-tui v0.1.0\nFinished release [optimized] target(s) in 2.3s",
            "stderr": "",
            "exit_code": 0
        }),
    ));
    state.push_tool_grouped(tool_with_data(
        "tool-bash-2",
        "bash",
        1200,
        false,
        serde_json::json!({ "command": "npm run test" }),
        serde_json::json!({
            "stdout": "PASS src/utils.test.ts\nPASS src/components.test.ts",
            "stderr": "FAIL src/api.test.ts\n  AssertionError: expected 200 but got 500\n  at Object.<anonymous> (src/api.test.ts:45:12)\n  at processTicksAndRejections",
            "exit_code": 1
        }),
    ));

    // EDIT_FILE TOOL
    state.push_tool_grouped(tool_with_data(
        "tool-edit-1",
        "edit_file",
        87,
        true,
        serde_json::json!({
            "file_path": "crates/locus_tui/src/view.rs",
            "old_string": "fn render() {\n    let x = 1;\n}",
            "new_string": "fn render() {\n    let x = 2;\n    let y = 3;\n}"
        }),
        serde_json::json!({ "success": true }),
    ));

    // CREATE_FILE TOOL
    state.push_tool_grouped(tool_with_data(
        "tool-create-1",
        "create_file",
        12,
        true,
        serde_json::json!({ "file_path": "src/lib.rs" }),
        serde_json::json!({
            "lines_written": 156,
            "success": true
        }),
    ));

    // UNDO_EDIT TOOL
    state.push_tool_grouped(tool_with_data(
        "tool-undo-1",
        "undo_edit",
        8,
        true,
        serde_json::json!({ "file_path": "crates/locus_tui/src/view.rs" }),
        serde_json::json!({ "restored": true }),
    ));

    // READ TOOL (file)
    state.push_tool_grouped(tool_with_data(
        "tool-read-1",
        "read",
        3,
        true,
        serde_json::json!({
            "file_path": "crates/locus_tui/src/main.rs",
            "offset": 0,
            "limit": 50
        }),
        serde_json::json!({
            "lines": 42,
            "range": "0-42"
        }),
    ));

    // ===== READ TOOL (directory) =====
    state.push_tool_grouped(tool_with_data(
        "tool-read-2",
        "read",
        2,
        true,
        serde_json::json!({ "dir_path": "crates/locus_tui/src/messages" }),
        serde_json::json!({
            "entries": ["mod.rs", "tool.rs", "ai_message.rs", "user.rs", "error.rs"],
            "count": 5
        }),
    ));

    // GLOB TOOL
    state.push_tool_grouped(tool_with_data(
        "tool-glob-1",
        "glob",
        15,
        true,
        serde_json::json!({ "pattern": "**/*.rs" }),
        serde_json::json!({
            "matches": [
                "src/main.rs",
                "src/lib.rs",
                "src/view.rs",
                "src/state.rs",
                "src/preview.rs",
                "src/theme/mod.rs",
                "src/messages/mod.rs"
            ],
            "count": 7
        }),
    ));

    // GREP TOOL
    state.push_tool_grouped(tool_with_data(
        "tool-grep-1",
        "grep",
        89,
        true,
        serde_json::json!({ "pattern": "fn render" }),
        serde_json::json!({
            "matches": [
                { "file": "src/view.rs", "line": 45, "content": "fn render_chat(state: &TuiState) {" },
                { "file": "src/view.rs", "line": 120, "content": "fn render_header(palette: &LocusPalette) {" },
                { "file": "src/components.rs", "line": 12, "content": "fn render_spinner() {" }
            ],
            "files": 2,
            "total": 3
        }),
    ));

    // FINDER TOOL
    state.push_tool_grouped(tool_with_data(
        "tool-finder-1",
        "finder",
        45,
        true,
        serde_json::json!({ "query": "tool render" }),
        serde_json::json!({
            "results": 12
        }),
    ));

    // HANDOFF TOOL
    state.push_tool_grouped(tool_with_data(
        "tool-handoff-1",
        "handoff",
        1500,
        true,
        serde_json::json!({
            "goal": "Implement a comprehensive tool UI spec with per-tool rendering, preview lines, and status summaries for all tool types in the locus_tui crate"
        }),
        serde_json::json!({
            "summary": "Completed implementation of all per-tool rendering modules with tests"
        }),
    ));

    // TASK_LIST TOOL
    state.push_tool_grouped(tool_with_data(
        "tool-tasklist-1",
        "task_list",
        5,
        true,
        serde_json::json!({ "action": "get" }),
        serde_json::json!({
            "tasks": [
                { "content": "Implement bash preview", "status": "completed" },
                { "content": "Implement edit_file preview", "status": "completed" },
                { "content": "Implement glob preview", "status": "in_progress" }
            ],
            "count": 3
        }),
    ));

    // WEB_AUTOMATION (fetch)
    state.push_tool_grouped(tool_with_data(
        "tool-fetch-1",
        "web_fetch",
        340,
        true,
        serde_json::json!({ "url": "https://docs.rs/ratatui/latest/ratatui/" }),
        serde_json::json!({ "success": true }),
    ));

    // ===== WEB_AUTOMATION (search) =====
    state.push_tool_grouped(tool_with_data(
        "tool-search-1",
        "web_search",
        890,
        true,
        serde_json::json!({ "query": "ratatui terminal ui rust examples" }),
        serde_json::json!({
            "results": 15
        }),
    ));

    // META-TOOLS
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
    state.push_meta_tool(MetaToolMessage::running(
        MetaToolKind::ToolExplain,
        Some("describe tool layout".to_string()),
    ));

    // EDIT DIFF (attached to edit tool)
    let edit_tool = tool_with_data(
        "tool-edit-diff",
        "edit_file",
        182,
        true,
        serde_json::json!({
            "file_path": "crates/locus_tui/src/view.rs"
        }),
        serde_json::json!({ "success": true }),
    );
    state.push_tool_grouped(edit_tool);
    state.insert_edit_diff_after_tool(
        "tool-edit-diff",
        EditDiffMessage {
            path: "crates/locus_tui/src/view.rs".to_string(),
            old_content: sample_old_file(),
            new_content: sample_new_file(),
            tool_id: Some("tool-edit-diff".to_string()),
        },
    );

    // MEMORY
    state.push_memory(MemoryMessage::recall(
        "tui design direction and transcript spacing",
        4,
    ));
    state.push_memory(MemoryMessage::store(
        "project:locuscodes_preview",
        "decision",
        "Preview mode should launch with seeded chat data and no runtime dependencies.",
    ));

    // ERRORS
    state.push_error(
        "Preview runtime error sample: LocusGraph transport failed while refreshing memory context."
            .to_string(),
        Some("09:42".to_string()),
    );
    state.push_error(
        "Runtime warning: tool group produced the expected diff but low confidence (channel meta)."
            .to_string(),
        Some("09:43".to_string()),
    );

    // STREAMING STATE
    state.push_user(
        "Can I inspect secondary screens too?".to_string(),
        Some("09:43".to_string()),
    );
    state.push_ai(
        "Yes. Use Ctrl+D for runtime logs, Ctrl+W for web automation, and --onboarding if you want the setup screen first."
            .to_string(),
        Some("09:43".to_string()),
    );

    state.is_streaming = true;
    state.current_think_text =
        "Comparing transcript spacing with the new chrome system.\nPreparing a final response with grouped tools, memory, and diff output."
            .to_string();

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
        assert!(state.is_streaming);
        assert!(!state.current_think_text.is_empty());
        assert!(matches!(state.messages[0], ChatItem::Separator(_)));
    }

    #[test]
    fn preview_state_can_start_on_onboarding() {
        let state = preview_state(true, Appearance::Light);
        assert_eq!(state.screen, Screen::Onboarding);
        assert!(!state.messages.is_empty());
    }
}
