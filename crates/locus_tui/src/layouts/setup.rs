//! Layout and rendering for the interactive setup wizard.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::layouts::{
    background_style, border_focused_style, border_style, danger_style, success_style,
    text_muted_style, text_style,
};
use crate::setup::{
    PROVIDERS, footer_hints, graph_choice_label, mask_for_input, mask_preview,
    provider_description, provider_env_var, provider_label, setup_progress,
};
use crate::state::{SetupStep, TuiState};
use crate::utils::padding;

const TOTAL_STEPS: usize = 5;

pub fn draw_setup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let palette = &state.palette;
    frame.render_widget(
        Block::default().style(background_style(palette.background)),
        area,
    );

    let card = centered_card(area);
    let card_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style(palette.border))
        .style(background_style(palette.surface_background));
    let inner = card_block.inner(card);
    frame.render_widget(card_block, card);

    let zones = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(10),
        Constraint::Length(2),
    ])
    .split(inner);

    draw_header(frame, zones[0], state);
    draw_content(frame, zones[1], state);
    draw_footer(frame, zones[2], state);
}

fn centered_card(area: Rect) -> Rect {
    let width = area.width.min(74).max(48);
    let height = area.height.min(24).max(14);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn draw_header(frame: &mut Frame, area: Rect, state: &TuiState) {
    let palette = &state.palette;
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(border_style(palette.border))
        .style(background_style(palette.status_bar_background));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let left = "locus.codes";
    let progress = progress_text(state.setup.step);
    let gap = inner.width.saturating_sub(
        (UnicodeWidthStr::width(left) + UnicodeWidthStr::width(progress.as_str())) as u16,
    );
    let line = Line::from(vec![
        Span::styled(
            left.to_string(),
            text_style(palette.text).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(gap as usize)),
        progress_span(state.setup.step, palette),
    ]);
    frame.render_widget(
        Paragraph::new(line).style(background_style(palette.status_bar_background)),
        inner,
    );
}

fn progress_text(step: SetupStep) -> String {
    let progress = setup_progress(step);
    let mut text = String::new();
    for idx in 0..TOTAL_STEPS {
        let symbol = if progress == TOTAL_STEPS {
            "●"
        } else if idx < progress {
            "●"
        } else if idx == progress {
            "◉"
        } else {
            "○"
        };
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(symbol);
    }
    text
}

fn progress_span(step: SetupStep, palette: &crate::theme::LocusPalette) -> Span<'static> {
    Span::styled(progress_text(step), text_style(palette.accent))
}

fn draw_content(frame: &mut Frame, area: Rect, state: &TuiState) {
    let palette = &state.palette;
    let block = Block::default().style(background_style(palette.surface_background));
    let inner = padding(block.inner(area), 2, 1);
    frame.render_widget(block, area);

    match state.setup.step {
        SetupStep::Welcome => draw_welcome(frame, inner, state),
        SetupStep::SelectProvider => draw_provider_select(frame, inner, state),
        SetupStep::EnterApiKey => draw_api_key_step(frame, inner, state),
        SetupStep::LocusGraphChoice => draw_graph_choice(frame, inner, state),
        SetupStep::LocusGraphUrl => draw_graph_input(frame, inner, state, SetupStep::LocusGraphUrl),
        SetupStep::LocusGraphSecret => {
            draw_graph_input(frame, inner, state, SetupStep::LocusGraphSecret)
        }
        SetupStep::LocusGraphId => draw_graph_input(frame, inner, state, SetupStep::LocusGraphId),
        SetupStep::Confirm => draw_confirm(frame, inner, state),
        SetupStep::Done => draw_done(frame, inner, state),
    }
}

fn draw_footer(frame: &mut Frame, area: Rect, state: &TuiState) {
    let palette = &state.palette;
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(border_style(palette.border))
        .style(background_style(palette.status_bar_background));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let hints = footer_hints(state.setup.step);
    let mut spans = Vec::new();
    for (idx, (key, action)) in hints.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(
                "  |  ".to_string(),
                text_muted_style(palette.border_variant),
            ));
        }
        spans.push(Span::styled(
            (*key).to_string(),
            text_style(palette.accent).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            (*action).to_string(),
            text_muted_style(palette.text_muted),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(background_style(palette.status_bar_background)),
        inner,
    );
}

fn draw_welcome(frame: &mut Frame, area: Rect, state: &TuiState) {
    let palette = &state.palette;
    let lines = vec![
        Line::from(""),
        centered_line(
            vec![
                Span::styled("▐█▌".to_string(), text_style(palette.accent)),
                Span::raw("  "),
                Span::styled(
                    "locus.codes".to_string(),
                    text_style(palette.text).add_modifier(Modifier::BOLD),
                ),
            ],
            area.width,
        ),
        centered_line(
            vec![Span::styled(
                "Terminal-native coding agent with memory".to_string(),
                text_muted_style(palette.text_muted),
            )],
            area.width,
        ),
        Line::from(""),
        Line::from(vec![Span::styled(
            "This wizard sets up the minimum config to get started.",
            text_style(palette.text),
        )]),
        Line::from(vec![Span::styled(
            "You'll need an API key for at least one LLM provider.",
            text_muted_style(palette.text_muted),
        )]),
        error_line(state),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn draw_provider_select(frame: &mut Frame, area: Rect, state: &TuiState) {
    let sections = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(1),
    ])
    .split(area);

    draw_step_header(
        frame,
        sections[0],
        "Choose your LLM provider",
        "Which provider's models should the agent use?",
    );

    for idx in 0..PROVIDERS.len() {
        let row_area = sections[1 + idx];
        let selected = idx == state.setup.provider_cursor;
        draw_selection_row(
            frame,
            row_area,
            provider_label(PROVIDERS[idx].0),
            provider_description(idx),
            selected,
            state,
        );
    }

    if let Some(line) = error_line_option(state) {
        frame.render_widget(Paragraph::new(line), sections[4]);
    }
}

fn draw_api_key_step(frame: &mut Frame, area: Rect, state: &TuiState) {
    let provider = state
        .setup
        .selected_provider
        .as_deref()
        .unwrap_or_else(|| provider_id_for_index(state.setup.provider_cursor));
    let label = provider_label(provider);
    let env_var = provider_env_var(provider).unwrap_or("API_KEY");

    let sections = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(area);

    draw_step_header(
        frame,
        sections[0],
        &format!("Enter your {} API key", label),
        "Paste your key - it stays local in ~/.locus.",
    );
    draw_input_field(
        frame,
        sections[1],
        env_var,
        &mask_for_input(&state.setup.api_key),
        true,
        state,
    );
    if let Some(line) = error_line_option(state) {
        frame.render_widget(Paragraph::new(line), sections[2]);
    }
}

fn draw_graph_choice(frame: &mut Frame, area: Rect, state: &TuiState) {
    let sections = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(1),
    ])
    .split(area);

    draw_step_header(
        frame,
        sections[0],
        "Memory",
        "LocusGraph gives the agent memory across sessions.",
    );

    for idx in 0..2 {
        draw_selection_row(
            frame,
            sections[1 + idx],
            if idx == 0 { "Yes" } else { "Skip" },
            graph_choice_label(idx),
            idx == state.setup.graph_choice_cursor,
            state,
        );
    }

    if let Some(line) = error_line_option(state) {
        frame.render_widget(Paragraph::new(line), sections[3]);
    }
}

fn draw_graph_input(frame: &mut Frame, area: Rect, state: &TuiState, step: SetupStep) {
    let sections = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(area);

    let (title, description, label, value, secret) = match step {
        SetupStep::LocusGraphUrl => (
            "LocusGraph server URL",
            "The gRPC endpoint for your LocusGraph instance.",
            "LOCUSGRAPH_SERVER_URL",
            state.setup.graph_url.as_str(),
            false,
        ),
        SetupStep::LocusGraphSecret => (
            "LocusGraph secret",
            "Your agent authentication secret.",
            "LOCUSGRAPH_AGENT_SECRET",
            state.setup.graph_secret.as_str(),
            true,
        ),
        SetupStep::LocusGraphId => (
            "Graph ID",
            "Namespace for this agent's memory.",
            "LOCUSGRAPH_GRAPH_ID",
            state.setup.graph_id.as_str(),
            false,
        ),
        _ => return,
    };

    draw_step_header(frame, sections[0], title, description);
    let display = if secret {
        mask_for_input(value)
    } else {
        value.to_string()
    };
    draw_input_field(frame, sections[1], label, &display, secret, state);
    if let Some(line) = error_line_option(state) {
        frame.render_widget(Paragraph::new(line), sections[2]);
    }
}

fn draw_confirm(frame: &mut Frame, area: Rect, state: &TuiState) {
    let palette = &state.palette;
    let sections = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(8),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(area);

    draw_step_header(
        frame,
        sections[0],
        "Review your configuration",
        "Everything looks good?",
    );

    let card = Block::default()
        .title(Span::styled(
            " Configuration ".to_string(),
            text_style(palette.text).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(border_style(palette.border_variant))
        .style(background_style(palette.elevated_surface_background));
    let inner = padding(card.inner(sections[1]), 1, 1);
    frame.render_widget(card, sections[1]);

    let provider = state
        .setup
        .selected_provider
        .as_deref()
        .map(provider_label)
        .unwrap_or("Not selected");
    let graph_status = if state.setup.configure_graph {
        "Configured"
    } else {
        "Skipped"
    };
    let graph_style = if state.setup.configure_graph {
        success_style(palette.success)
    } else {
        text_muted_style(palette.text_muted)
    };
    let lines = vec![
        summary_line(inner.width, "Provider", provider, state),
        summary_line(
            inner.width,
            "API Key",
            &mask_preview(&state.setup.api_key),
            state,
        ),
        Line::from(vec![
            Span::styled(
                format!("{:<12}", "LocusGraph"),
                text_muted_style(palette.text_muted),
            ),
            Span::styled(graph_status.to_string(), graph_style),
        ]),
        summary_line(
            inner.width,
            "URL",
            if state.setup.configure_graph {
                &state.setup.graph_url
            } else {
                "-"
            },
            state,
        ),
        summary_line(
            inner.width,
            "Graph ID",
            if state.setup.configure_graph {
                &state.setup.graph_id
            } else {
                "-"
            },
            state,
        ),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Saved to ".to_string(),
                text_muted_style(palette.text_muted),
            ),
            Span::styled("~/.locus/locus.db".to_string(), text_style(palette.text)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

    if let Some(line) = error_line_option(state) {
        frame.render_widget(Paragraph::new(line), sections[2]);
    }
}

fn draw_done(frame: &mut Frame, area: Rect, state: &TuiState) {
    let palette = &state.palette;
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(centered_line(
            vec![Span::styled(
                "✓".to_string(),
                success_style(palette.success).add_modifier(Modifier::BOLD),
            )],
            area.width,
        )),
        sections[0],
    );

    let saved_line = if let Some(shimmer) = &state.setup.done_shimmer {
        centered_line(
            shimmer.styled_spans_with_palette("Configuration saved.", palette),
            area.width,
        )
    } else {
        centered_line(
            vec![Span::styled(
                "Configuration saved.".to_string(),
                success_style(palette.success),
            )],
            area.width,
        )
    };
    frame.render_widget(Paragraph::new(saved_line), sections[1]);
    frame.render_widget(
        Paragraph::new(centered_line(
            vec![Span::styled(
                "Press Enter to start chatting.",
                text_style(palette.text),
            )],
            area.width,
        )),
        sections[2],
    );
}

fn draw_step_header(frame: &mut Frame, area: Rect, title: &str, description: &str) {
    let title_line = Line::from(vec![Span::styled(
        title.to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    )]);
    let description_line = Line::from(description.to_string());
    let lines = vec![
        title_line.patch_style(Style::default()),
        Line::from(""),
        description_line,
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn draw_selection_row(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    description: &str,
    selected: bool,
    state: &TuiState,
) {
    let palette = &state.palette;
    let bg = if selected {
        background_style(palette.element_selected)
    } else {
        background_style(palette.surface_background)
    };
    frame.render_widget(Block::default().style(bg), area);

    let prefix = if selected { "› " } else { "  " };
    let line = Line::from(vec![
        Span::styled(
            prefix.to_string(),
            if selected {
                text_style(palette.accent)
            } else {
                text_muted_style(palette.text_muted)
            },
        ),
        Span::styled(
            format!("{:<12}", label),
            text_style(palette.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            description.to_string(),
            text_muted_style(palette.text_muted),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_input_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    state: &TuiState,
) {
    let palette = &state.palette;
    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", label),
            text_style(palette.text),
        ))
        .borders(Borders::ALL)
        .border_style(border_focused_style(palette.border_focused))
        .style(background_style(palette.elevated_surface_background));
    let inner = padding(block.inner(area), 1, 0);
    frame.render_widget(block, area);

    let mut spans = vec![Span::styled(value.to_string(), text_style(palette.text))];
    if focused {
        spans.push(Span::styled("▌".to_string(), text_style(palette.accent)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), inner);
}

fn summary_line(width: u16, label: &str, value: &str, state: &TuiState) -> Line<'static> {
    let palette = &state.palette;
    let label_width = 12usize;
    let available = width as usize;
    let max_value = available.saturating_sub(label_width + 2);
    let display = truncate(value, max_value);
    Line::from(vec![
        Span::styled(
            format!("{:<12}", label),
            text_muted_style(palette.text_muted),
        ),
        Span::styled(display, text_style(palette.text)),
    ])
}

fn truncate(value: &str, max_width: usize) -> String {
    if value.chars().count() <= max_width {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_width.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn centered_line(spans: Vec<Span<'static>>, width: u16) -> Line<'static> {
    let content_width: usize = spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
        .sum();
    let pad = width.saturating_sub(content_width as u16) / 2;
    let mut padded = vec![Span::raw(" ".repeat(pad as usize))];
    padded.extend(spans);
    Line::from(padded)
}

fn error_line_option(state: &TuiState) -> Option<Line<'static>> {
    state.setup.error_message.as_ref().map(|message| {
        Line::from(vec![Span::styled(
            message.clone(),
            danger_style(state.palette.danger),
        )])
    })
}

fn error_line(state: &TuiState) -> Line<'static> {
    error_line_option(state).unwrap_or_else(|| Line::from(""))
}

fn provider_id_for_index(index: usize) -> &'static str {
    PROVIDERS[index.min(PROVIDERS.len().saturating_sub(1))].0
}
