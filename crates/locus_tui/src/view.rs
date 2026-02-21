//! TUI view: header (fixed top), scrollable chat body, shortcut + input (fixed bottom).

use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::layouts::{
    ChatsLayout, INPUT_ICON, block_for_input_bordered,
    main_splits_with_padding, render_header, shortcut_inner_rect, shortcut_line, text_style,
    vertical_split, HEADER_STATUS_READY, HEADER_TITLE,
    text_muted_style,
};
use crate::messages::tool::ToolCallStatus;
use crate::messages::{ai_message, ai_think_message, error, meta_tool, tool, user};
use crate::state::{ChatItem, Screen, TuiState};
use crate::utils::{format_duration, LEFT_PADDING};

/// Draw the full TUI: main chat, debug traces, or web automation depending on state.screen.
pub fn draw(frame: &mut Frame, state: &mut TuiState, area: Rect) {
    match state.screen {
        Screen::DebugTraces => draw_debug_traces(frame, state, area),
        Screen::WebAutomation => {
            crate::web_automation::draw_web_automation(frame, &mut state.web_automation, area, &state.palette);
        }
        Screen::Main => draw_main(frame, state, area),
    }
}

/// Runtime logs screen: scrollable list of tracing output. Ctrl+D to close.
fn draw_debug_traces(frame: &mut Frame, state: &mut TuiState, area: Rect) {
    use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

    let palette = &state.palette;
    let title = " Runtime logs (Ctrl+D to close) ";
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(crate::layouts::border_style(palette.border))
        .style(crate::layouts::background_style(palette.background));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content_height = state.trace_lines.len();
    let viewport_height = inner.height as usize;
    let max_scroll = content_height.saturating_sub(viewport_height);
    state.trace_scroll = state.trace_scroll.min(max_scroll);
    let offset = state.trace_scroll;

    let lines: Vec<Line> = state
        .trace_lines
        .iter()
        .skip(offset)
        .take(viewport_height)
        .map(|s| {
            Line::from(ratatui::text::Span::styled(
                s.clone(),
                crate::layouts::text_muted_style(palette.text_muted),
            ))
        })
        .collect();
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Main chat view: header, scrollable chat body, shortcut + input fixed bottom.
fn draw_main(frame: &mut Frame, state: &mut TuiState, area: Rect) {
    let splits = main_splits_with_padding(area);
    let palette = &state.palette;

    // ---- Header (fixed at top) ----
    let status = if state.status.is_empty() {
        HEADER_STATUS_READY
    } else {
        state.status.as_str()
    };
    let has_error = state.status.to_lowercase().contains("error")
        || state.status.to_lowercase().contains("failed");
    render_header(
        frame,
        splits.header,
        palette,
        HEADER_TITLE,
        status,
        state.is_streaming,
        has_error,
    );

    // ---- Body: scrollable chat ----
    let chat = ChatsLayout::new(splits.body);
    let width = chat.inner.width as usize;
    let viewport_height = chat.inner.height as usize;

    let spacer = Line::from("");
    let cursor_visible = (state.frame_count / 5).is_multiple_of(2); // 500ms blink at 100ms tick

    let mut all_lines: Vec<Line> = if state.cache_dirty {
        let has_running_tool = state.messages.iter().any(|m| {
            matches!(m, ChatItem::Tool(t) if matches!(t.status, ToolCallStatus::Running))
        });
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
            if !lines.is_empty() {
                lines.push(spacer.clone());
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
                ChatItem::User(m) => {
                    lines.extend(user::user_message_lines(m, palette, width));
                    i += 1;
                }
                ChatItem::Ai(m) => {
                    lines.extend(ai_message::ai_message_lines(
                        m, palette, width, false, true,
                    ));
                    i += 1;
                }
                ChatItem::Think(m) => {
                    lines.extend(ai_think_message::think_message_lines(
                        m, palette, width, false, true, None,
                    ));
                    i += 1;
                }
                ChatItem::MetaTool(m) => {
                    lines.push(meta_tool::meta_tool_line(m, palette));
                    i += 1;
                }
                ChatItem::Error(m) => {
                    lines.extend(error::error_message_lines(m, palette, width));
                    i += 1;
                }
                ChatItem::Separator(label) => {
                    let sep = Line::from(vec![ratatui::text::Span::styled(
                        format!("── {} ──", label),
                        crate::layouts::text_muted_style(palette.text_disabled),
                    )]);
                    lines.push(sep);
                    i += 1;
                }
                ChatItem::ToolGroup(tools) => {
                    let total = tools.len();
                    let running_count = tools.iter().filter(|t| matches!(t.status, ToolCallStatus::Running)).count();
                    let all_done = running_count == 0;

                    // Group header line
                    let header_text = if all_done {
                        let total_ms: u64 = tools.iter().map(|t| match &t.status {
                            ToolCallStatus::Done { duration_ms, .. } => *duration_ms,
                            _ => 0,
                        }).max().unwrap_or(0);
                        format!("⫘ {} tools  {}", total, format_duration(std::time::Duration::from_millis(total_ms)))
                    } else {
                        format!("⫘ {} tools running", total)
                    };

                    let header_style = if all_done {
                        text_muted_style(palette.text_muted)
                    } else {
                        text_style(palette.accent)
                    };

                    lines.push(Line::from(vec![
                        ratatui::text::Span::raw(LEFT_PADDING),
                        ratatui::text::Span::styled(header_text, header_style),
                    ]));

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
            text: think_text,
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
    {
        if !all_lines.is_empty() {
            all_lines.push(spacer.clone());
        }
        let mut shimmer = crate::animation::Shimmer::new();
        shimmer.tick();
        let spans = shimmer.styled_spans_with_palette("⋯", palette);
        all_lines.push(Line::from(spans));
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
            text: ai_text,
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
        let title = "locus.codes";
        let sub = "Type a message to begin.";
        let title_line = Line::from(vec![ratatui::text::Span::styled(
            title.to_string(),
            text_style(palette.text),
        )]);
        let sub_line = Line::from(vec![ratatui::text::Span::styled(
            sub.to_string(),
            crate::layouts::text_muted_style(palette.text_muted),
        )]);
        let para = Paragraph::new(vec![
            Line::from(""),
            title_line,
            Line::from(""),
            sub_line,
        ])
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(para, chat.inner);
    } else {
        let paragraph = Paragraph::new(visible).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, chat.inner);
    }

    // Scrollbar when content exceeds viewport
    if content_height > viewport_height && !state.messages.is_empty() {
        let track = palette.scrollbar_track_background;
        let thumb = palette.scrollbar_thumb_hover_background;
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
        let track_style = ratatui::style::Style::default().bg(crate::layouts::rgb_to_color(track));
        frame.render_widget(
            ratatui::widgets::Block::default().style(track_style),
            scrollbar_rect,
        );
        let thumb_rect = Rect {
            x: scrollbar_rect.x,
            y: scrollbar_rect.y + thumb_y,
            width: 1,
            height: thumb_height,
        };
        let thumb_style = ratatui::style::Style::default().bg(crate::layouts::rgb_to_color(thumb));
        frame.render_widget(
            ratatui::widgets::Block::default().style(thumb_style),
            thumb_rect,
        );
    }

    // ---- Footer: input block + shortcut ----
    let (input_rect, shortcut_rect) = vertical_split(splits.footer, 3);

    let block = block_for_input_bordered(palette, true);
    let inner = block.inner(input_rect);
    frame.render_widget(block, input_rect);

    let placeholder = "Ask anything…";
    let (icon_style, content_style) = if state.input_buffer.is_empty() {
        (text_style(palette.accent), text_style(palette.text_placeholder))
    } else {
        (text_style(palette.success), text_style(palette.text))
    };
    let input_line = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(INPUT_ICON.to_string(), icon_style),
        ratatui::text::Span::styled(
            if state.input_buffer.is_empty() {
                placeholder.to_string()
            } else {
                state.input_buffer.clone()
            },
            content_style,
        ),
    ]);
    frame.render_widget(Paragraph::new(input_line), inner);

    // Cursor: display width (unicode-width) for position
    let icon_width = INPUT_ICON.width();
    let before_cursor = &state.input_buffer[..state.input_cursor.min(state.input_buffer.len())];
    let cursor_col_offset = before_cursor.width();
    let cursor_col = (inner.x + icon_width as u16 + cursor_col_offset as u16).min(inner.x + inner.width);
    frame.set_cursor_position((cursor_col, inner.y));

    let shortcut_inner = shortcut_inner_rect(shortcut_rect);
    frame.render_widget(
        Paragraph::new(shortcut_line(
            palette,
            state.is_streaming,
            !state.input_buffer.is_empty(),
        )),
        shortcut_inner,
    );
}
