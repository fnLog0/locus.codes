//! View rendering for web automation screen.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::theme::LocusPalette;
use crate::web_automation::state::{AutomationStatus, WebAutomationState};
use crate::layouts::{text_style, text_muted_style, border_style, background_style};
use crate::utils::LEFT_PADDING;

/// Draw the web automation screen.
pub fn draw_web_automation(frame: &mut Frame, state: &mut WebAutomationState, area: Rect, palette: &LocusPalette) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(3),  // URL/Goal input
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
    let (status_text, status_color) = match state.status {
        AutomationStatus::Idle => ("Idle", palette.text_muted),
        AutomationStatus::Starting => ("Starting...", palette.accent),
        AutomationStatus::Running => ("Running", palette.accent),
        AutomationStatus::Completed => ("Completed", palette.success),
        AutomationStatus::Failed => ("Failed", palette.danger),
    };

    let elapsed = if state.is_running() {
        format!(" · {}", state.elapsed())
    } else if let Some(ms) = state.duration_ms {
        format!(" · {}ms", ms)
    } else {
        String::new()
    };

    let title = Line::from(vec![
        Span::styled(" Web Automation ", text_style(palette.text)),
        Span::styled(
            format!("[{}{}]", status_text, elapsed),
            text_style(status_color),
        ),
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::BOTTOM)
        .border_style(border_style(palette.border));

    frame.render_widget(block, area);
}

fn draw_input_section(frame: &mut Frame, area: Rect, state: &WebAutomationState, palette: &LocusPalette) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // URL display
    let url_text = if state.url.is_empty() {
        "URL: (not set)".to_string()
    } else {
        format!("URL: {}", state.url)
    };
    let url_line = Line::from(vec![
        Span::styled(
            url_text,
            if state.url.is_empty() {
                text_muted_style(palette.text_muted)
            } else {
                text_style(palette.text)
            },
        ),
    ]);
    let url_para = Paragraph::new(url_line);
    frame.render_widget(url_para, chunks[0]);

    // Goal display
    let goal_text = if state.goal.is_empty() {
        "Goal: (not set)".to_string()
    } else {
        let max_len = 50;
        if state.goal.len() > max_len {
            format!("Goal: {}...", &state.goal[..max_len])
        } else {
            format!("Goal: {}", state.goal)
        }
    };
    let goal_line = Line::from(vec![
        Span::styled(
            goal_text,
            if state.goal.is_empty() {
                text_muted_style(palette.text_muted)
            } else {
                text_style(palette.text)
            },
        ),
    ]);
    let goal_para = Paragraph::new(goal_line);
    frame.render_widget(goal_para, chunks[1]);
}

fn draw_progress_section(frame: &mut Frame, area: Rect, state: &mut WebAutomationState, palette: &LocusPalette) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style(palette.border))
        .style(background_style(palette.background));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let viewport_height = inner.height as usize;

    // Build content lines
    let mut lines: Vec<Line> = Vec::new();

    // If idle, show help
    if state.status == AutomationStatus::Idle && state.progress_messages.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Press ", text_muted_style(palette.text_muted)),
            Span::styled("Enter", text_style(palette.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" to start a new automation task", text_muted_style(palette.text_muted)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Example: ", text_muted_style(palette.text_muted)),
            Span::styled("https://example.com", text_style(palette.text)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Goal: ", text_muted_style(palette.text_muted)),
            Span::styled("Extract the page title", text_style(palette.text)),
        ]));
    } else {
        // Show streaming URL if available
        if let Some(ref url) = state.streaming_url {
            lines.push(Line::from(vec![
                Span::styled("🔴 Live: ", text_style(palette.danger)),
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
                Span::styled(LEFT_PADDING, Style::default()),
                Span::styled(format!("{}{}", prefix, msg), style),
            ]));
        }

        // Show result if completed
        if let Some(ref result) = state.result {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(LEFT_PADDING, Style::default()),
                Span::styled("Result:", text_style(palette.text).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(""));

            // Pretty-print JSON
            if let Ok(json_str) = serde_json::to_string_pretty(result) {
                for line in json_str.lines() {
                    lines.push(Line::from(vec![
                        Span::styled(LEFT_PADDING, Style::default()),
                        Span::styled(line.to_string(), text_muted_style(palette.text_muted)),
                    ]));
                }
            }
        }

        // Show error if failed
        if let Some(ref error) = state.error {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(LEFT_PADDING, Style::default()),
                Span::styled("Error:", text_style(palette.danger).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::styled(LEFT_PADDING, Style::default()),
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
            ("Ctrl+W", "Back"),
            ("Ctrl+C", "Cancel"),
        ]
    } else {
        vec![
            ("Ctrl+W", "Back"),
            ("Enter", "New"),
            ("Esc", "Quit"),
        ]
    };

    let spans: Vec<Span> = shortcuts
        .iter()
        .flat_map(|(key, action)| {
            vec![
                Span::styled(" ", Style::default()),
                Span::styled(*key, text_style(palette.accent)),
                Span::styled(format!(" {} ", action), text_muted_style(palette.text_muted)),
            ]
        })
        .collect();

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
