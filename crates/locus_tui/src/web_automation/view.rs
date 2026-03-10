//! View rendering for web automation screen.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::theme::LocusPalette;
use crate::web_automation::state::{AutomationStatus, WebAutomationState};
use crate::layouts::{background_style, border_style, render_header, text_muted_style, text_style};
use crate::utils::LEFT_PADDING;

/// Draw the web automation screen.
pub fn draw_web_automation(frame: &mut Frame, state: &mut WebAutomationState, area: Rect, palette: &LocusPalette) {
    frame.render_widget(Block::default().style(background_style(palette.background)), area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(4),  // URL/Goal summary
            Constraint::Min(10),    // Progress/Result
            Constraint::Length(1),  // Shortcuts
        ])
        .split(area);

    draw_header(frame, chunks[0], state, palette);
    draw_input_section(frame, chunks[1], state, palette);
    draw_progress_section(frame, chunks[2], state, palette);
    draw_shortcuts(frame, chunks[3], state, palette);
}

fn draw_header(frame: &mut Frame, area: Rect, state: &WebAutomationState, palette: &LocusPalette) {
    let (status_text, active, has_error) = match state.status {
        AutomationStatus::Idle => ("Idle".to_string(), false, false),
        AutomationStatus::Starting => ("Preparing browser".to_string(), true, false),
        AutomationStatus::Running => ("Running".to_string(), true, false),
        AutomationStatus::Completed => ("Completed".to_string(), false, false),
        AutomationStatus::Failed => ("Failed".to_string(), false, true),
    };

    let status = if state.is_running() {
        format!("{}  {}", status_text, state.elapsed())
    } else if let Some(ms) = state.duration_ms {
        format!("{}  {}ms", status_text, ms)
    } else {
        status_text
    };

    render_header(
        frame,
        area,
        palette,
        "locus.codes",
        "web automation",
        status.as_str(),
        active,
        has_error,
    );
}

fn draw_input_section(frame: &mut Frame, area: Rect, state: &WebAutomationState, palette: &LocusPalette) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style(palette.border))
        .style(background_style(palette.surface_background));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let url_text = if state.url.is_empty() {
        "(not set)".to_string()
    } else {
        state.url.clone()
    };
    let goal_text = if state.goal.is_empty() {
        "(not set)".to_string()
    } else if state.goal.len() > 72 {
        format!("{}…", &state.goal[..71])
    } else {
        state.goal.clone()
    };
    let run_text = state
        .run_id
        .as_deref()
        .map(str::to_string)
        .unwrap_or_else(|| "waiting".to_string());

    let lines = vec![
        Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled("▏ ".to_string(), text_muted_style(palette.border_variant)),
            Span::styled("url   ".to_string(), text_muted_style(palette.text_muted)),
            Span::styled(
                url_text,
                if state.url.is_empty() {
                    text_muted_style(palette.text_muted)
                } else {
                    text_style(palette.text)
                },
            ),
        ]),
        Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled("▏ ".to_string(), text_muted_style(palette.border_variant)),
            Span::styled("goal  ".to_string(), text_muted_style(palette.text_muted)),
            Span::styled(
                goal_text,
                if state.goal.is_empty() {
                    text_muted_style(palette.text_muted)
                } else {
                    text_style(palette.text)
                },
            ),
        ]),
        Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled("▏ ".to_string(), text_muted_style(palette.border_variant)),
            Span::styled("run   ".to_string(), text_muted_style(palette.text_muted)),
            Span::styled(run_text, text_muted_style(palette.text_muted)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_progress_section(frame: &mut Frame, area: Rect, state: &mut WebAutomationState, palette: &LocusPalette) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style(palette.border))
        .style(background_style(palette.elevated_surface_background));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let viewport_height = inner.height as usize;

    // Build content lines
    let mut lines: Vec<Line> = Vec::new();

    // If idle, show help
    if state.status == AutomationStatus::Idle && state.progress_messages.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled("● ".to_string(), text_style(palette.accent)),
            Span::styled("browser automation is idle".to_string(), text_style(palette.text)),
        ]));
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled(
                "  press Enter to launch a sample run",
                text_muted_style(palette.text_muted),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled("  url", text_muted_style(palette.text_muted)),
            Span::styled("  https://example.com", text_style(palette.text)),
        ]));
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled("  goal", text_muted_style(palette.text_muted)),
            Span::styled("  Extract the page title", text_style(palette.text)),
        ]));
    } else {
        // Show streaming URL if available
        if let Some(ref url) = state.streaming_url {
            lines.push(Line::from(vec![
                Span::raw(LEFT_PADDING),
                Span::styled("▏ ".to_string(), text_style(palette.info)),
                Span::styled("live browser  ".to_string(), text_style(palette.info)),
                Span::styled(url, text_style(palette.accent)),
            ]));
            lines.push(Line::from(""));
        }

        // Show progress messages
        for msg in &state.progress_messages {
            let prefix = if msg.starts_with("Error") {
                "✗ "
            } else if msg.starts_with("Completed") {
                "✓ "
            } else if msg.starts_with("Started") {
                "▶ "
            } else {
                "→ "
            };
            let style = if msg.starts_with("Error") {
                text_style(palette.danger)
            } else if msg.starts_with("Completed") {
                text_style(palette.success)
            } else {
                text_muted_style(palette.text_muted)
            };
            lines.push(Line::from(vec![
                Span::raw(LEFT_PADDING),
                Span::styled("│ ".to_string(), text_muted_style(palette.border_variant)),
                Span::styled(format!("{}{}", prefix, msg), style),
            ]));
        }

        // Show result if completed
        if let Some(ref result) = state.result {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw(LEFT_PADDING),
                Span::styled("▏ ".to_string(), text_muted_style(palette.border_variant)),
                Span::styled("Result:", text_style(palette.text).add_modifier(Modifier::BOLD)),
            ]));

            // Pretty-print JSON
            if let Ok(json_str) = serde_json::to_string_pretty(result) {
                for line in json_str.lines() {
                    lines.push(Line::from(vec![
                        Span::raw(LEFT_PADDING),
                        Span::raw("  "),
                        Span::styled(line.to_string(), text_muted_style(palette.text_muted)),
                    ]));
                }
            }
        }

        // Show error if failed
        if let Some(ref error) = state.error {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw(LEFT_PADDING),
                Span::styled("▏ ".to_string(), text_style(palette.danger)),
                Span::styled("Error:", text_style(palette.danger).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::raw(LEFT_PADDING),
                Span::raw("  "),
                Span::styled(error.clone(), text_style(palette.danger)),
            ]));
        }
    }

    // Calculate scroll
    let content_height = lines.len();
    let max_scroll = content_height.saturating_sub(viewport_height);
    state.scroll = state.scroll.min(max_scroll);
    let offset = max_scroll.saturating_sub(state.scroll);

    let visible: Vec<Line> = lines
        .into_iter()
        .skip(offset)
        .take(viewport_height)
        .collect();

    let paragraph = Paragraph::new(visible).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn draw_shortcuts(frame: &mut Frame, area: Rect, state: &WebAutomationState, palette: &LocusPalette) {
    let shortcuts = if state.is_running() {
        vec![
            ("Ctrl+W", "back"),
            ("Ctrl+C", "cancel"),
            ("↑↓", "scroll"),
        ]
    } else {
        vec![
            ("Ctrl+W", "back"),
            ("Enter", "sample run"),
            ("r", "reset"),
        ]
    };

    let mut spans: Vec<Span> = Vec::new();
    for (idx, (key, action)) in shortcuts.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled("  ·  ".to_string(), text_muted_style(palette.text_disabled)));
        }
        spans.push(Span::styled((*key).to_string(), text_style(palette.text)));
        spans.push(Span::styled(
            format!(": {}", action),
            text_muted_style(palette.text_muted),
        ));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
