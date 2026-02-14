use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use locus_core::RuntimeEvent;
use crate::detect_term::AdaptiveColors;

/// Task Board - displays active task, status, history, and tool activity
pub fn render_task_board_view(f: &mut Frame, area: Rect, events: &[RuntimeEvent]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(0),
            Constraint::Min(4),
            Constraint::Length(2),
        ])
        .split(area);

    let header_chunk = chunks[0];
    let _active_chunk = chunks[1];
    let content_chunk = chunks[2];
    let footer_chunk = chunks[3];

    // Header: title + event count
    let header_text = vec![
        Line::from(vec![
            Span::styled("  Task Board ", Style::default().fg(AdaptiveColors::cyan()).add_modifier(Modifier::BOLD)),
            Span::styled(" · ", Style::default().fg(AdaptiveColors::dark_gray())),
            Span::styled(format!("{} events", events.len()), Style::default().fg(AdaptiveColors::yellow())),
        ]),
    ];
    f.render_widget(Paragraph::new(header_text), header_chunk);

    // Active task: show if latest task is still running (TaskStarted without TaskCompleted/TaskFailed)
    let active_task_lines = active_task_summary(events);
    let (_active_area, content_area) = if active_task_lines.is_empty() {
        (Rect::default(), content_chunk)
    } else {
        let active_height = (active_task_lines.len() as u16).saturating_add(4); // lines + block borders
        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(active_height), Constraint::Min(1)])
            .split(content_chunk);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(AdaptiveColors::green()))
            .title(" Active task ");
        let block_inner = block.inner(inner[0]);
        f.render_widget(block, inner[0]);
        f.render_widget(Paragraph::new(active_task_lines).wrap(Wrap { trim: true }), block_inner);
        (inner[0], inner[1])
    };

    // Event stream (last 20 events, newest first)
    let mut lines = Vec::new();
    let display_events: Vec<_> = events.iter().rev().take(20).collect();
    for (i, ev) in display_events.iter().enumerate() {
        lines.extend(format_event(ev, i + 1));
        lines.push(Line::from(""));
    }

    if events.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  No events yet. ", Style::default().fg(AdaptiveColors::dark_gray())),
            Span::styled("Type a prompt below and press Enter to start.", Style::default().fg(AdaptiveColors::text())),
        ]));
    } else if lines.is_empty() {
        lines.push(Line::from(Span::styled("  (No displayable events)", Style::default().fg(AdaptiveColors::dark_gray()))));
    }

    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AdaptiveColors::blue()))
        .title(" Event stream ");
    let content_inner = content_block.inner(content_area);
    f.render_widget(content_block, content_area);
    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }),
        content_inner,
    );

    // Footer
    let footer_text = vec![
        Line::from(vec![
            Span::styled(" ? ", Style::default().fg(AdaptiveColors::yellow())),
            Span::styled("shortcuts  ", Style::default().fg(AdaptiveColors::dark_gray())),
            Span::styled(" 1–6 ", Style::default().fg(AdaptiveColors::yellow())),
            Span::styled("views  ", Style::default().fg(AdaptiveColors::dark_gray())),
            Span::styled(" Enter ", Style::default().fg(AdaptiveColors::yellow())),
            Span::styled("send prompt", Style::default().fg(AdaptiveColors::dark_gray())),
        ]),
    ];
    f.render_widget(
        Paragraph::new(footer_text).alignment(Alignment::Center),
        footer_chunk,
    );
}

/// Build summary lines for the current active task (last TaskStarted with no completion/failure yet)
fn active_task_summary(events: &[RuntimeEvent]) -> Vec<Line<'_>> {
    let completed_or_failed: std::collections::HashSet<&str> = events
        .iter()
        .filter_map(|ev| {
            match ev {
                RuntimeEvent::TaskCompleted { task_id, .. } | RuntimeEvent::TaskFailed { task_id, .. } => {
                    Some(task_id.as_str())
                }
                _ => None,
            }
        })
        .collect();
    let mut last_started: Option<(&str, &str, &locus_core::Mode)> = None;
    for ev in events.iter() {
        if let RuntimeEvent::TaskStarted { task_id, prompt, mode } = ev {
            last_started = Some((task_id.as_str(), prompt.as_str(), mode));
        }
    }
    let mut lines = Vec::new();
    if let Some((id, prompt, mode)) = last_started {
        if completed_or_failed.contains(id) {
            return lines;
        }
        lines.push(Line::from(vec![
            Span::styled("  Prompt: ", Style::default().fg(AdaptiveColors::dark_gray())),
            Span::styled(truncate_string(prompt, 70), Style::default().fg(AdaptiveColors::text())),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ID: ", Style::default().fg(AdaptiveColors::dark_gray())),
            Span::styled(shorten_uuid(id), Style::default().fg(AdaptiveColors::dark_magenta())),
            Span::styled("  · ", Style::default().fg(AdaptiveColors::dark_gray())),
            Span::styled(format!("{:?}", mode), Style::default().fg(AdaptiveColors::cyan())),
        ]));
    }
    lines
}

fn format_event(event: &RuntimeEvent, index: usize) -> Vec<Line<'_>> {
    let mut lines = Vec::new();

    match event {
        RuntimeEvent::TaskStarted { task_id, prompt, mode } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("TASK STARTED", Style::default().fg(AdaptiveColors::green()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" [{}]", mode), Style::default().fg(AdaptiveColors::cyan())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Prompt: ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(truncate_string(prompt, 50), Style::default().fg(AdaptiveColors::text())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  ID: ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(shorten_uuid(task_id), Style::default().fg(AdaptiveColors::dark_magenta())),
            ]));
        }
        RuntimeEvent::TaskCompleted { task_id, summary, duration_ms } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("TASK COMPLETED", Style::default().fg(AdaptiveColors::green()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" ({}ms)", duration_ms), Style::default().fg(AdaptiveColors::yellow())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Summary: ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(truncate_string(summary, 60), Style::default().fg(AdaptiveColors::text())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  ID: ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(shorten_uuid(task_id), Style::default().fg(AdaptiveColors::dark_magenta())),
            ]));
        }
        RuntimeEvent::TaskFailed { task_id, error, step } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("TASK FAILED", Style::default().fg(AdaptiveColors::red()).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Error: ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(truncate_string(error, 60), Style::default().fg(AdaptiveColors::red())),
            ]));
            if let Some(s) = step {
                lines.push(Line::from(vec![
                    Span::styled("  Step: ", Style::default().fg(AdaptiveColors::dark_gray())),
                    Span::styled(s, Style::default().fg(AdaptiveColors::yellow())),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled("  ID: ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(shorten_uuid(task_id), Style::default().fg(AdaptiveColors::dark_magenta())),
            ]));
        }
        RuntimeEvent::ToolCalled { tool, args, .. } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("TOOL CALL", Style::default().fg(AdaptiveColors::cyan()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {}", tool), Style::default().fg(AdaptiveColors::blue())),
            ]));
            let args_str = args.to_string();
            if !args_str.is_empty() && args_str != "null" && args_str != "{}" {
                lines.push(Line::from(vec![
                    Span::styled("  Args: ", Style::default().fg(AdaptiveColors::dark_gray())),
                    Span::styled(truncate_string(args_str.trim(), 56), Style::default().fg(AdaptiveColors::text())),
                ]));
            }
        }
        RuntimeEvent::ToolResult { tool, success, result, duration_ms } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("TOOL RESULT", Style::default().fg(AdaptiveColors::cyan()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {}", tool), Style::default().fg(AdaptiveColors::blue())),
                Span::styled(format!(" ({}ms)", duration_ms), Style::default().fg(AdaptiveColors::yellow())),
            ]));
            let result_str = result.to_string();
            let trimmed = result_str.trim();
            if *success {
                lines.push(Line::from(vec![
                    Span::styled("  ✓ ", Style::default().fg(AdaptiveColors::green())),
                    Span::styled(truncate_string(trimmed, 68), Style::default().fg(AdaptiveColors::text())),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  ✗ ", Style::default().fg(AdaptiveColors::red())),
                    Span::styled(truncate_string(trimmed, 68), Style::default().fg(AdaptiveColors::red())),
                ]));
            }
        }
        RuntimeEvent::ModeChanged { old_mode, new_mode } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("MODE CHANGED", Style::default().fg(AdaptiveColors::orange()).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(format!("{:?}", old_mode), Style::default().fg(AdaptiveColors::red())),
                Span::styled(" → ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(format!("{:?}", new_mode), Style::default().fg(AdaptiveColors::green())),
            ]));
        }
        RuntimeEvent::AgentSpawned { agent_id: _, agent_type, task } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("AGENT SPAWNED", Style::default().fg(AdaptiveColors::dark_magenta()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", agent_type), Style::default().fg(AdaptiveColors::blue())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Task: ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(truncate_string(task, 60), Style::default().fg(AdaptiveColors::text())),
            ]));
        }
        RuntimeEvent::AgentCompleted { agent_id: _, status, result } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("AGENT COMPLETED", Style::default().fg(AdaptiveColors::dark_magenta()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", status), Style::default().fg(AdaptiveColors::green())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(truncate_string(result, 65), Style::default().fg(AdaptiveColors::text())),
            ]));
        }
        RuntimeEvent::DiffGenerated { files, hunks_count } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("DIFF GENERATED", Style::default().fg(AdaptiveColors::orange()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} hunks, {} file(s)", hunks_count, files.len()), Style::default().fg(AdaptiveColors::yellow())),
            ]));
        }
        RuntimeEvent::DiffApproved { files } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("DIFF APPROVED", Style::default().fg(AdaptiveColors::green()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} file(s)", files.len()), Style::default().fg(AdaptiveColors::text())),
            ]));
        }
        RuntimeEvent::DiffRejected { files: _, reason } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("DIFF REJECTED", Style::default().fg(AdaptiveColors::red()).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(truncate_string(reason, 60), Style::default().fg(AdaptiveColors::red())),
            ]));
        }
        RuntimeEvent::TestResult { passed, failed, total, output } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("TEST RESULT", Style::default().fg(AdaptiveColors::cyan()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {}/{} passed", passed, total), Style::default().fg(if *failed == 0 { AdaptiveColors::green() } else { AdaptiveColors::red() })),
            ]));
            if !output.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().fg(AdaptiveColors::dark_gray())),
                    Span::styled(truncate_string(output.trim(), 60), Style::default().fg(AdaptiveColors::text())),
                ]));
            }
        }
        RuntimeEvent::MemoryRecalled { locus_count, top_confidence } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("MEMORY RECALLED", Style::default().fg(AdaptiveColors::dark_magenta()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} loci, top conf {:.2}", locus_count, top_confidence), Style::default().fg(AdaptiveColors::text())),
            ]));
        }
        RuntimeEvent::MemoryStored { event_kind, context_id } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("MEMORY STORED", Style::default().fg(AdaptiveColors::dark_magenta()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} {}", event_kind, context_id), Style::default().fg(AdaptiveColors::text())),
            ]));
        }
        RuntimeEvent::CommitCreated { hash, message, files: _ } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("COMMIT", Style::default().fg(AdaptiveColors::green()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", &hash[..hash.len().min(8)]), Style::default().fg(AdaptiveColors::dark_magenta())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(truncate_string(message, 55), Style::default().fg(AdaptiveColors::text())),
            ]));
        }
        RuntimeEvent::DebugIteration { iteration, failure_summary } => {
            lines.push(Line::from(vec![
                Span::styled(format!("[{:02}] ", index), Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled("DEBUG ITERATION", Style::default().fg(AdaptiveColors::orange()).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" #{} ", iteration), Style::default().fg(AdaptiveColors::yellow())),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default().fg(AdaptiveColors::dark_gray())),
                Span::styled(truncate_string(failure_summary, 60), Style::default().fg(AdaptiveColors::text())),
            ]));
        }
    }

    lines
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn shorten_uuid(uuid: &str) -> String {
    if uuid.len() > 12 {
        format!("{}...{}", &uuid[..8], &uuid[uuid.len()-4..])
    } else {
        uuid.to_string()
    }
}

/// Plan View - placeholder for execution DAG visualization (Phase 3+)
pub fn render_plan_view(f: &mut Frame, area: Rect) {
    let text = vec![
        "".into(),
        "  Plan View".into(),
        "  —".into(),
        "".into(),
        "  Execution DAG visualization".into(),
        "  (Coming in Phase 3)".into(),
        "".into(),
        "  Press [Esc] to return to Task Board".into(),
    ];
    f.render_widget(
        ratatui::widgets::Paragraph::new(text)
            .block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(ratatui::style::Style::default().fg(crate::detect_term::AdaptiveColors::blue()))
            ),
        area,
    );
}

/// Agents View - placeholder for active subagent cards (Phase 3+)
pub fn render_agents_view(f: &mut Frame, area: Rect) {
    let text = vec![
        "".into(),
        "  Agents View".into(),
        "  —".into(),
        "".into(),
        "  Active subagent cards".into(),
        "  (Coming in Phase 3)".into(),
        "".into(),
        "  Press [Esc] to return to Task Board".into(),
    ];
    f.render_widget(
        ratatui::widgets::Paragraph::new(text)
            .block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(ratatui::style::Style::default().fg(crate::detect_term::AdaptiveColors::blue()))
            ),
        area,
    );
}

/// Diff Review View - placeholder for diff review (Phase 2+)
pub fn render_diff_review_view(f: &mut Frame, area: Rect) {
    let text = vec![
        "".into(),
        "  Diff Review".into(),
        "  —".into(),
        "".into(),
        "  Unified diff viewer with approval".into(),
        "  (Coming in Phase 2)".into(),
        "".into(),
        "  Press [Esc] to return to Task Board".into(),
    ];
    f.render_widget(
        ratatui::widgets::Paragraph::new(text)
            .block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(ratatui::style::Style::default().fg(crate::detect_term::AdaptiveColors::blue()))
            ),
        area,
    );
}

/// Logs View - placeholder for command output display
pub fn render_logs_view(f: &mut Frame, area: Rect) {
    let text = vec![
        "".into(),
        "  Logs View".into(),
        "  —".into(),
        "".into(),
        "  ToolBus command output".into(),
        "  (Placeholder for now)".into(),
        "".into(),
        "  Press [Esc] to return to Task Board".into(),
    ];
    f.render_widget(
        ratatui::widgets::Paragraph::new(text)
            .block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(ratatui::style::Style::default().fg(crate::detect_term::AdaptiveColors::blue()))
            ),
        area,
    );
}

/// Memory Trace View - placeholder for memory injection display (Phase 4+)
pub fn render_memory_trace_view(f: &mut Frame, area: Rect) {
    let text = vec![
        "".into(),
        "  Memory Trace".into(),
        "  —".into(),
        "".into(),
        "  LocusGraph memory recall/injection".into(),
        "  (Coming in Phase 4)".into(),
        "".into(),
        "  Press [Esc] to return to Task Board".into(),
    ];
    f.render_widget(
        ratatui::widgets::Paragraph::new(text)
            .block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(ratatui::style::Style::default().fg(crate::detect_term::AdaptiveColors::blue()))
            ),
        area,
    );
}
