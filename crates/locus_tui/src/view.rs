//! TUI view: header (fixed top), scrollable chat body, shortcut + input (fixed bottom).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::layouts::{
    ChatsLayout, INPUT_ICON, INPUT_PADDING_H, background_style, block_for_input_bordered,
    main_splits_with_padding_and_footer_height, render_header, shortcut_inner_rect, shortcut_line, text_style,
    vertical_split, HEADER_STATUS_READY, HEADER_TITLE,
    CHAT_MESSAGE_SPACING, text_muted_style, warning_style,
};
use crate::messages::tool::ToolCallStatus;
use crate::messages::{ai_message, ai_think_message, edit_diff, error, memory, meta_tool, tool, user};
use crate::state::{ChatItem, Screen, TuiState};
use crate::messages::edit_diff::DIFF_PAGE_SIZE;
use crate::utils::collapse_repeated_chars;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LivePhase {
    Ready,
    Reviewing,
    Preparing,
    Thinking,
    Responding,
    Tooling,
}

impl LivePhase {
    fn is_active(self) -> bool {
        matches!(
            self,
            Self::Preparing | Self::Thinking | Self::Responding | Self::Tooling
        )
    }

    fn header_label(self) -> &'static str {
        match self {
            Self::Ready => HEADER_STATUS_READY,
            Self::Reviewing => "Reviewing",
            Self::Preparing => "Preparing",
            Self::Thinking => "Thinking",
            Self::Responding => "Responding",
            Self::Tooling => "Running tools",
        }
    }

    fn footer_label(self) -> Option<&'static str> {
        match self {
            Self::Preparing => Some("Preparing"),
            Self::Thinking => Some("Thinking"),
            Self::Responding => Some("Responding"),
            Self::Tooling => Some("Running tools"),
            Self::Ready | Self::Reviewing => None,
        }
    }
}

fn has_running_tools(messages: &[ChatItem]) -> bool {
    messages.iter().any(|message| match message {
        ChatItem::Tool(tool) => matches!(tool.status, ToolCallStatus::Running),
        ChatItem::ToolGroup(group) => group
            .iter()
            .any(|tool| matches!(tool.status, ToolCallStatus::Running)),
        _ => false,
    })
}

fn has_ai_history(messages: &[ChatItem]) -> bool {
    messages
        .iter()
        .any(|message| matches!(message, ChatItem::Ai(_)))
}

fn live_phase(state: &TuiState) -> LivePhase {
    if !state.current_ai_text.is_empty() {
        LivePhase::Responding
    } else if !state.current_think_text.is_empty() {
        LivePhase::Thinking
    } else if has_running_tools(&state.messages) {
        LivePhase::Tooling
    } else if state.is_streaming {
        LivePhase::Preparing
    } else if !state.auto_scroll {
        LivePhase::Reviewing
    } else {
        LivePhase::Ready
    }
}

fn phase_glyph(frame_count: u64) -> &'static str {
    match frame_count % 8 {
        0 | 1 => "●",
        2 | 3 => "◔",
        4 | 5 => "◑",
        _ => "◕",
    }
}

fn header_section_label(state: &TuiState, phase: LivePhase) -> &'static str {
    if !state.auto_scroll {
        "manual review"
    } else if phase.is_active() {
        "live session"
    } else {
        "main workspace"
    }
}

fn header_status_text(state: &TuiState, phase: LivePhase) -> String {
    let status = state.status.trim();
    let status_lower = status.to_ascii_lowercase();
    let status_is_error = status_lower.contains("error") || status_lower.contains("failed");

    if status_is_error {
        status.to_string()
    } else if phase.is_active() {
        phase.header_label().to_string()
    } else if !status.is_empty() {
        status.to_string()
    } else {
        phase.header_label().to_string()
    }
}

fn preparing_indicator_lines(palette: &crate::theme::LocusPalette, frame_count: u64) -> Vec<Line<'static>> {
    let rail = Span::styled("▏ ".to_string(), text_muted_style(palette.text_muted));
    let warning = warning_style(palette.warning);
    let muted = text_muted_style(palette.text_muted);

    vec![
        Line::from(vec![
            rail.clone(),
            Span::styled(format!("{} ", phase_glyph(frame_count)), warning),
            Span::styled("preparing response".to_string(), text_style(palette.text)),
        ]),
        Line::from(vec![
            rail,
            Span::raw("  "),
            Span::styled(
                "waiting for the first token",
                muted,
            ),
        ]),
    ]
}

fn empty_state_lines(palette: &crate::theme::LocusPalette) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("●".to_string(), text_style(palette.accent)),
            Span::raw(" "),
            Span::styled("locus.codes".to_string(), text_style(palette.text)),
        ]),
        Line::from(vec![Span::styled(
            "quiet terminal workspace for code, tools, and memory".to_string(),
            text_muted_style(palette.text_muted),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Try".to_string(), text_style(palette.accent)),
            Span::styled(
                ": review the last changes".to_string(),
                text_muted_style(palette.text_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled("Try".to_string(), text_style(palette.accent)),
            Span::styled(
                ": explain this crate".to_string(),
                text_muted_style(palette.text_muted),
            ),
        ]),
        Line::from(vec![
            Span::styled("Try".to_string(), text_style(palette.accent)),
            Span::styled(
                ": patch the failing test".to_string(),
                text_muted_style(palette.text_muted),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter".to_string(), text_style(palette.text)),
            Span::styled(": send", text_muted_style(palette.text_muted)),
            Span::styled("  ·  ".to_string(), text_muted_style(palette.text_disabled)),
            Span::styled("Ctrl+D".to_string(), text_style(palette.text)),
            Span::styled(": logs", text_muted_style(palette.text_muted)),
            Span::styled("  ·  ".to_string(), text_muted_style(palette.text_disabled)),
            Span::styled("Ctrl+N".to_string(), text_style(palette.text)),
            Span::styled(": new session", text_muted_style(palette.text_muted)),
        ]),
    ]
}

const INPUT_BORDER_HEIGHT: u16 = 2;
const INPUT_SHORTCUT_HEIGHT: u16 = 1;
const INPUT_MAX_CONTENT_LINES: usize = 6;

#[derive(Debug, Clone)]
struct InputVisualState {
    lines: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
}

fn wrap_input_for_display(text: &str, cursor_byte: usize, line_width: usize) -> InputVisualState {
    if line_width == 0 {
        return InputVisualState {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
        };
    }

    let cursor_byte = cursor_byte.min(text.len());
    let mut lines = vec![String::new()];
    let mut current_width = 0usize;
    let mut cursor_line = 0usize;
    let mut cursor_col = 0usize;
    let mut seen_cursor = false;

    for (byte_idx, ch) in text.char_indices() {
        if !seen_cursor && byte_idx == cursor_byte {
            cursor_line = lines.len() - 1;
            cursor_col = current_width;
            seen_cursor = true;
        }

        if ch == '\n' {
            lines.push(String::new());
            current_width = 0;
            continue;
        }

        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
        if current_width + ch_width > line_width && current_width > 0 {
            lines.push(String::new());
            current_width = 0;
        }
        lines.last_mut().unwrap().push(ch);
        current_width += ch_width;
    }

    if !seen_cursor {
        cursor_line = lines.len() - 1;
        cursor_col = current_width;
    }

    InputVisualState {
        lines: if lines.is_empty() { vec![String::new()] } else { lines },
        cursor_line,
        cursor_col,
    }
}

fn input_visual_state(buffer: &str, cursor_byte: usize, line_width: usize) -> InputVisualState {
    let wrapped = wrap_input_for_display(buffer, cursor_byte, line_width);
    if wrapped.lines.is_empty() {
        return InputVisualState {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
        };
    }

    if wrapped.lines.len() <= INPUT_MAX_CONTENT_LINES {
        return wrapped;
    }

    let visible_start = wrapped
        .cursor_line
        .saturating_add(1)
        .saturating_sub(INPUT_MAX_CONTENT_LINES)
        .min(wrapped.lines.len().saturating_sub(INPUT_MAX_CONTENT_LINES));

    InputVisualState {
        lines: wrapped.lines[visible_start..visible_start + INPUT_MAX_CONTENT_LINES].to_vec(),
        cursor_line: wrapped.cursor_line.saturating_sub(visible_start),
        cursor_col: wrapped.cursor_col,
    }
}

fn input_footer_height(area_width: u16, buffer: &str, cursor_byte: usize) -> u16 {
    let icon_width = INPUT_ICON.width() as u16;
    let inner_width = area_width
        .saturating_sub(2)
        .saturating_sub(INPUT_PADDING_H.saturating_mul(2));
    let text_width = inner_width.saturating_sub(icon_width) as usize;
    let visual = input_visual_state(buffer, cursor_byte, text_width.max(1));
    INPUT_BORDER_HEIGHT + visual.lines.len() as u16 + INPUT_SHORTCUT_HEIGHT
}

fn message_spacing_between(previous: &ChatItem, current: &ChatItem) -> usize {
    match (previous, current) {
        (ChatItem::Tool(_), ChatItem::EditDiff(_))
        | (ChatItem::ToolGroup(_), ChatItem::EditDiff(_))
        | (ChatItem::Separator(_), _) => 0,
        _ => CHAT_MESSAGE_SPACING,
    }
}

fn separator_line(label: &str, palette: &crate::theme::LocusPalette, width: usize) -> Line<'static> {
    let separator_style = text_muted_style(palette.text_disabled);
    let tail_style = text_muted_style(palette.border_variant);
    let label_width = label.chars().count();
    let tail_len = width
        .saturating_sub(2 + 2 + label_width + 1)
        .clamp(4, 32);

    Line::from(vec![
        Span::raw("  "),
        Span::styled("· ".to_string(), separator_style),
        Span::styled(label.to_string(), separator_style),
        Span::raw(" "),
        Span::styled("─".repeat(tail_len), tail_style),
    ])
}

/// Draw the full TUI: main chat, onboarding, debug traces, or web automation depending on state.screen.
pub fn draw(frame: &mut Frame, state: &mut TuiState, area: Rect) {
    match state.screen {
        Screen::Onboarding => draw_onboarding(frame, state, area),
        Screen::DebugTraces => draw_debug_traces(frame, state, area),
        Screen::WebAutomation => {
            crate::web_automation::draw_web_automation(frame, &mut state.web_automation, area, &state.palette);
        }
        Screen::Main => draw_main(frame, state, area),
    }
}

/// Onboarding screen: configure API keys and related settings. Shown when no LLM key is set.
fn draw_onboarding(frame: &mut Frame, state: &mut TuiState, area: Rect) {
    use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

    let palette = &state.palette;
    frame.render_widget(Block::default().style(background_style(palette.background)), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(1)])
        .split(area);

    render_header(
        frame,
        chunks[0],
        palette,
        HEADER_TITLE,
        "first run",
        "Configuration",
        false,
        false,
    );

    let body = Block::default()
        .borders(Borders::ALL)
        .border_style(crate::layouts::border_style(palette.border))
        .style(background_style(palette.surface_background));
    let inner = body.inner(chunks[1]);
    frame.render_widget(body, chunks[1]);

    let normal = text_style(palette.text);
    let muted = text_muted_style(palette.text_muted);
    let accent = text_style(palette.accent);
    let disabled = text_muted_style(palette.text_disabled);

    let lines = vec![
        Line::from(vec![
            Span::styled("● ".to_string(), accent),
            Span::styled("configure at least one provider".to_string(), normal),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "the agent needs an API key before the chat can start",
                muted,
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("locus config api".to_string(), normal),
            Span::styled("  add anthropic, zai, or tinyfish", muted),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("locus config graph".to_string(), normal),
            Span::styled("  set LocusGraph URL and secret", muted),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("locus config --help".to_string(), normal),
            Span::styled("  inspect all configuration options", muted),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("● ".to_string(), accent),
            Span::styled("config location".to_string(), normal),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("~/.locus/env".to_string(), normal),
            Span::styled("  source it after updates, then restart the TUI", muted),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("● ".to_string(), accent),
            Span::styled("next".to_string(), normal),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Enter".to_string(), normal),
            Span::styled("  continue to chat", muted),
            Span::styled("  ·  ".to_string(), disabled),
            Span::styled("q".to_string(), normal),
            Span::styled("  quit", muted),
            Span::styled("  ·  ".to_string(), disabled),
            Span::styled("locus tui --onboarding".to_string(), normal),
            Span::styled("  show this again", muted),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Ctrl+D".to_string(), normal),
            Span::styled(": logs", muted),
            Span::styled("  ·  ".to_string(), disabled),
            Span::styled("docs/prompts.md".to_string(), normal),
            Span::styled(": prompt ideas", muted),
        ])),
        chunks[2],
    );
}

/// Runtime logs screen: scrollable list of tracing output. Ctrl+D to close.
fn draw_debug_traces(frame: &mut Frame, state: &mut TuiState, area: Rect) {
    use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

    let palette = &state.palette;
    frame.render_widget(Block::default().style(background_style(palette.background)), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(8), Constraint::Length(1)])
        .split(area);

    let status = if state.trace_lines.is_empty() {
        "No logs".to_string()
    } else {
        format!("{} lines", state.trace_lines.len())
    };
    render_header(
        frame,
        chunks[0],
        palette,
        HEADER_TITLE,
        "runtime logs",
        status.as_str(),
        false,
        false,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(crate::layouts::border_style(palette.border))
        .style(background_style(palette.surface_background));
    let inner = block.inner(chunks[1]);
    frame.render_widget(block, chunks[1]);

    let content_height = state.trace_lines.len();
    let viewport_height = inner.height as usize;
    let max_scroll = content_height.saturating_sub(viewport_height);
    state.trace_scroll = state.trace_scroll.min(max_scroll);
    let offset_from_top = max_scroll.saturating_sub(state.trace_scroll);

    let lines: Vec<Line> = if state.trace_lines.is_empty() {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("● ".to_string(), text_style(palette.accent)),
                Span::styled("runtime tracing is quiet".to_string(), text_style(palette.text)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  logs from the runtime and event stream appear here while the session is active",
                    text_muted_style(palette.text_muted),
                ),
            ]),
        ]
    } else {
        state
            .trace_lines
            .iter()
            .skip(offset_from_top)
            .take(viewport_height)
            .map(|line| {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("│ ".to_string(), text_muted_style(palette.border_variant)),
                    Span::styled(line.clone(), text_muted_style(palette.text_muted)),
                ])
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Esc".to_string(), text_style(palette.text)),
            Span::styled(": back", text_muted_style(palette.text_muted)),
            Span::styled("  ·  ".to_string(), text_muted_style(palette.text_disabled)),
            Span::styled("↑↓".to_string(), text_style(palette.text)),
            Span::styled(": scroll", text_muted_style(palette.text_muted)),
            Span::styled("  ·  ".to_string(), text_muted_style(palette.text_disabled)),
            Span::styled("PgUp/PgDn".to_string(), text_style(palette.text)),
            Span::styled(": faster", text_muted_style(palette.text_muted)),
        ])),
        chunks[2],
    );
}

/// Main chat view: header, scrollable chat body, shortcut + input fixed bottom.
fn draw_main(frame: &mut Frame, state: &mut TuiState, area: Rect) {
    let footer_height = input_footer_height(area.width, &state.input_buffer, state.input_cursor);
    let splits = main_splits_with_padding_and_footer_height(area, footer_height);
    let palette = &state.palette;
    let phase = live_phase(state);
    let header_status = header_status_text(state, phase);

    // ---- Header (fixed at top) ----
    let has_error = header_status.to_ascii_lowercase().contains("error")
        || header_status.to_ascii_lowercase().contains("failed");
    render_header(
        frame,
        splits.header,
        palette,
        HEADER_TITLE,
        header_section_label(state, phase),
        header_status.as_str(),
        phase.is_active(),
        has_error,
    );

    // ---- Body: scrollable chat ----
    let chat = ChatsLayout::new(splits.body);
    let width = chat.inner.width as usize;
    let viewport_height = chat.inner.height as usize;

    let spacer = Line::from("");
    let cursor_visible = (state.frame_count / 5).is_multiple_of(2); // 500ms blink at 100ms tick

    let mut all_lines: Vec<Line> = if state.cache_dirty {
        let has_running_tool = has_running_tools(&state.messages);
        if has_running_tool {
            state
                .tool_shimmer
                .get_or_insert_with(crate::animation::Shimmer::new)
                .tick();
        } else {
            state.tool_shimmer = None;
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_millis() as u64);

        let mut lines = Vec::new();
        let mut i = 0;
        while i < state.messages.len() {
            if i > 0 {
                for _ in 0..message_spacing_between(&state.messages[i - 1], &state.messages[i]) {
                    lines.push(spacer.clone());
                }
            }
            match &state.messages[i] {
                ChatItem::Tool(t) => {
                    let elapsed = t
                        .started_at_ms
                        .and_then(|s| now_ms.map(|n| n.saturating_sub(s)));
                    let name_spans = if matches!(t.status, ToolCallStatus::Running) {
                        state.tool_shimmer.as_ref().map(|sh| {
                            sh.styled_spans_with_palette(&t.tool_name, palette)
                        })
                    } else {
                        None
                    };
                    lines.extend(tool::tool_call_lines(
                        t,
                        palette,
                        elapsed,
                        name_spans,
                        false,
                    ));
                    i += 1;
                }
                ChatItem::EditDiff(d) => {
                    let start = if state.diff_page_message_index == Some(i) {
                        state.diff_page_offset
                    } else {
                        0
                    };
                    lines.extend(edit_diff::edit_diff_block_lines(
                        d,
                        palette,
                        width,
                        start,
                        DIFF_PAGE_SIZE,
                    ));
                    i += 1;
                }
                ChatItem::User(m) => {
                    lines.extend(user::user_message_lines(m, palette, width));
                    i += 1;
                }
                ChatItem::Ai(m) => {
                    let collapsed = ai_message::AiMessage {
                        text: collapse_repeated_chars(&m.text, 4),
                        timestamp: m.timestamp.clone(),
                    };
                    lines.extend(ai_message::ai_message_lines(
                        &collapsed, palette, width, false, true,
                    ));
                    i += 1;
                }
                ChatItem::Think(m) => {
                    let collapsed_think = ai_think_message::AiThinkMessage {
                        text: collapse_repeated_chars(&m.text, 4),
                        collapsed: m.collapsed,
                    };
                    lines.extend(ai_think_message::think_message_lines(
                        &collapsed_think, palette, width, false, true, None,
                    ));
                    i += 1;
                }
                ChatItem::MetaTool(m) => {
                    lines.extend(meta_tool::meta_tool_lines(m, palette));
                    i += 1;
                }
                ChatItem::Memory(m) => {
                    lines.push(memory::memory_line(m, palette));
                    i += 1;
                }
                ChatItem::Error(m) => {
                    lines.extend(error::error_message_lines(m, palette, width));
                    i += 1;
                }
                ChatItem::Separator(label) => {
                    lines.push(separator_line(label, palette, width));
                    i += 1;
                }
                ChatItem::ToolGroup(tools) => {
                    lines.push(tool::tool_group_header_line(tools, palette));

                    // Individual tools (indented, no spacer between them)
                    for t in tools {
                        let elapsed = t.started_at_ms
                            .and_then(|s| now_ms.map(|n| n.saturating_sub(s)));
                        let name_spans = if matches!(t.status, ToolCallStatus::Running) {
                            state.tool_shimmer.as_ref().map(|sh| {
                                sh.styled_spans_with_palette(&t.tool_name, palette)
                            })
                        } else {
                            None
                        };
                        lines.extend(tool::tool_call_lines(t, palette, elapsed, name_spans, true));
                    }
                    i += 1;
                }
            }
        }
        state.cached_lines = lines.clone();
        state.cache_dirty = false;
        lines
    } else {
        state.cached_lines.clone()
    };

    const STREAMING_DISPLAY_CAP: usize = 80_000;

    // Streaming thinking (cap length so we never build millions of lines if stream runs away)
    if !state.current_think_text.is_empty() {
        if !all_lines.is_empty() {
            all_lines.push(spacer.clone());
        }
        let think_len = state.current_think_text.chars().count();
        let think_text: String = if think_len > STREAMING_DISPLAY_CAP {
            state.current_think_text.chars().skip(think_len - STREAMING_DISPLAY_CAP).collect()
        } else {
            state.current_think_text.clone()
        };
        let think = ai_think_message::AiThinkMessage {
            text: collapse_repeated_chars(&think_text, 4),
            collapsed: false,
        };
        all_lines.extend(ai_think_message::think_message_lines(
            &think,
            palette,
            width,
            true,
            cursor_visible,
            Some(3), // show last 3 lines + "…" during stream
        ));
    }

    // Typing indicator when streaming but no content yet
    if state.is_streaming
        && state.current_ai_text.is_empty()
        && state.current_think_text.is_empty()
        && !has_running_tools(&state.messages)
    {
        if !all_lines.is_empty() {
            all_lines.push(spacer.clone());
        }
        all_lines.extend(preparing_indicator_lines(palette, state.frame_count));
    }

    // Streaming AI: cap text length and cap line count so rendering stays responsive (no TUI hang).
    const STREAMING_LINE_CAP: usize = 120;
    if !state.current_ai_text.is_empty() {
        if !all_lines.is_empty() {
            all_lines.push(spacer.clone());
        }
        let ai_len = state.current_ai_text.chars().count();
        let ai_text: String = if ai_len > STREAMING_DISPLAY_CAP {
            state.current_ai_text.chars().skip(ai_len - STREAMING_DISPLAY_CAP).collect()
        } else {
            state.current_ai_text.clone()
        };
        let ai = ai_message::AiMessage {
            text: collapse_repeated_chars(&ai_text, 4),
            timestamp: None,
        };
        let mut stream_lines = ai_message::ai_message_lines(
            &ai,
            palette,
            width,
            true,
            cursor_visible,
        );
        if stream_lines.len() > STREAMING_LINE_CAP {
            let tail_start = stream_lines.len() - STREAMING_LINE_CAP;
            let ellipsis = Line::from(ratatui::text::Span::styled(
                "…",
                crate::layouts::text_muted_style(palette.text_muted),
            ));
            all_lines.push(ellipsis);
            all_lines.extend(stream_lines.drain(tail_start..));
        } else {
            all_lines.extend(stream_lines);
        }
    }

    let content_height = all_lines.len();

    // Scroll clamp: state.scroll is "lines scrolled UP from bottom" (0 = at bottom).
    let max_scroll = content_height.saturating_sub(viewport_height);
    state.scroll = state.scroll.min(max_scroll);
    state.last_content_height = content_height;
    state.last_viewport_height = viewport_height;

    // Convert to offset from top: scroll=0 → show last lines, scroll=max → show first lines.
    let offset_from_top = max_scroll.saturating_sub(state.scroll);
    let visible: Vec<Line> = all_lines
        .into_iter()
        .skip(offset_from_top)
        .take(viewport_height)
        .collect();

    // Empty state: welcome when no messages and not streaming
    if state.messages.is_empty()
        && state.current_ai_text.is_empty()
        && state.current_think_text.is_empty()
        && !state.is_streaming
    {
        let para = Paragraph::new(empty_state_lines(palette))
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(para, chat.inner);
    } else {
        let paragraph = Paragraph::new(visible).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, chat.inner);
    }

    // Scrollbar when content exceeds viewport
    if content_height > viewport_height && !state.messages.is_empty() {
        let track_style = text_muted_style(palette.border_variant);
        let thumb_style = if state.auto_scroll {
            text_style(palette.scrollbar_thumb_hover_background)
        } else {
            text_style(palette.scrollbar_thumb_active)
        };
        let thumb_height = (((viewport_height as f64) * (viewport_height as f64)
            / (content_height as f64).max(1.0))
            .ceil() as u16)
            .max(1);
        // scroll=0 is bottom, scroll=max is top. Scrollbar thumb should be at
        // bottom when scroll=0, top when scroll=max. Use offset_from_top ratio.
        let scroll_ratio = if max_scroll == 0 {
            1.0
        } else {
            offset_from_top as f64 / max_scroll as f64
        };
        let thumb_y = (scroll_ratio * (viewport_height as f64 - thumb_height as f64)).round() as u16;
        let scrollbar_rect = Rect {
            x: chat.inner.x + chat.inner.width.saturating_sub(1),
            y: chat.inner.y,
            width: 1,
            height: chat.inner.height,
        };
        let scrollbar_lines: Vec<Line> = (0..viewport_height)
            .map(|idx| {
                let idx = idx as u16;
                if idx >= thumb_y && idx < thumb_y.saturating_add(thumb_height) {
                    Line::from(ratatui::text::Span::styled("▐", thumb_style))
                } else {
                    Line::from(ratatui::text::Span::styled("▏", track_style))
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(scrollbar_lines), scrollbar_rect);
    }

    // ---- Footer: input block + shortcut ----
    frame.render_widget(
        Block::default().style(background_style(palette.status_bar_background)),
        splits.footer,
    );
    let input_height = splits.footer.height.saturating_sub(INPUT_SHORTCUT_HEIGHT);
    let (input_rect, shortcut_rect) = vertical_split(splits.footer, input_height);

    let block = block_for_input_bordered(palette, true);
    let inner = block.inner(input_rect);
    frame.render_widget(block, input_rect);

    let placeholder = "Ask anything…";
    let (icon_style, content_style) = if state.input_buffer.is_empty() {
        (text_style(palette.accent), text_style(palette.text_placeholder))
    } else {
        (text_style(palette.success), text_style(palette.text))
    };
    let icon_width = INPUT_ICON.width();
    if state.input_buffer.is_empty() {
        let input_line = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(INPUT_ICON.to_string(), icon_style),
            ratatui::text::Span::styled(placeholder.to_string(), content_style),
        ]);
        frame.render_widget(Paragraph::new(input_line), inner);
        frame.set_cursor_position((inner.x + icon_width as u16, inner.y));
    } else {
        let text_width = inner.width.saturating_sub(icon_width as u16) as usize;
        let visual = input_visual_state(&state.input_buffer, state.input_cursor, text_width.max(1));
        let continuation_prefix = " ".repeat(icon_width);
        let input_lines: Vec<Line> = visual
            .lines
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                let prefix = if idx == 0 {
                    INPUT_ICON.to_string()
                } else {
                    continuation_prefix.clone()
                };
                Line::from(vec![
                    Span::styled(prefix, icon_style),
                    Span::styled(line.clone(), content_style),
                ])
            })
            .collect();
        frame.render_widget(Paragraph::new(input_lines), inner);

        let cursor_x = (inner.x + icon_width as u16 + visual.cursor_col as u16)
            .min(inner.x + inner.width.saturating_sub(1));
        let cursor_y = (inner.y + visual.cursor_line as u16)
            .min(inner.y + inner.height.saturating_sub(1));
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    let shortcut_inner = shortcut_inner_rect(shortcut_rect);
    frame.render_widget(
        Paragraph::new(shortcut_line(
            palette,
            phase.footer_label(),
            phase.footer_label().map(|_| phase_glyph(state.frame_count)),
            !state.input_buffer.is_empty(),
            state.diff_page_message_index.is_some(),
            has_ai_history(&state.messages),
        )),
        shortcut_inner,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{
        ai_message::AiMessage,
        tool::{EditDiffMessage, ToolCallMessage},
        user::UserMessage,
    };

    #[test]
    fn tool_and_diff_stay_attached() {
        let tool = ChatItem::Tool(ToolCallMessage::running("t1", "edit_file", None));
        let diff = ChatItem::EditDiff(EditDiffMessage {
            path: "src/main.rs".into(),
            old_content: "old".into(),
            new_content: "new".into(),
            tool_id: Some("t1".into()),
        });

        assert_eq!(message_spacing_between(&tool, &diff), 0);
    }

    #[test]
    fn separator_does_not_add_extra_gap_afterwards() {
        let separator = ChatItem::Separator("New session".into());
        let ai = ChatItem::Ai(AiMessage {
            text: "reply".into(),
            timestamp: None,
        });

        assert_eq!(message_spacing_between(&separator, &ai), 0);
    }

    #[test]
    fn regular_message_spacing_uses_default_gap() {
        let user = ChatItem::User(UserMessage {
            text: "hi".into(),
            timestamp: None,
        });
        let ai = ChatItem::Ai(AiMessage {
            text: "hello".into(),
            timestamp: None,
        });

        assert_eq!(message_spacing_between(&user, &ai), CHAT_MESSAGE_SPACING);
    }

    #[test]
    fn separator_line_uses_transcript_indent() {
        let palette = crate::theme::LocusPalette::locus_dark();
        let line = separator_line("New session", &palette, 80);

        assert_eq!(line.spans[0].content.as_ref(), "  ");
        assert!(line.spans.iter().any(|span| span.content.contains("New session")));
    }

    #[test]
    fn input_wraps_to_second_visual_line() {
        let visual = wrap_input_for_display("abcdefghijk", 11, 5);
        assert_eq!(visual.lines, vec!["abcde", "fghij", "k"]);
        assert_eq!(visual.cursor_line, 2);
        assert_eq!(visual.cursor_col, 1);
    }

    #[test]
    fn input_footer_grows_when_text_wraps() {
        let short = input_footer_height(40, "short", 5);
        let long = input_footer_height(24, "this input should wrap onto another line", 39);
        assert!(long > short);
    }
}
