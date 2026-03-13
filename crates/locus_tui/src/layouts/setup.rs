//! Layout and rendering for the interactive setup wizard.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    layouts::{
        background_style, border_focused_style, border_style, danger_style, rgb_to_color,
        success_style, text_muted_style, text_style,
    },
    state::{SetupProvider, SetupState, SetupStep, TuiState},
    theme::LocusPalette,
};

pub fn draw_setup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let palette = &state.palette;
    let card = centered_rect(area);
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style(palette.border))
        .style(background_style(palette.background));
    let inner = outer.inner(card);
    frame.render_widget(outer, card);

    let zones = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(10),
        Constraint::Length(2),
    ])
    .split(inner);

    draw_header(frame, zones[0], &state.setup, palette);
    draw_content(frame, zones[1], state, palette);
    draw_footer(frame, zones[2], state.setup.step, palette);
}

fn draw_header(frame: &mut Frame, area: Rect, setup: &SetupState, palette: &LocusPalette) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(border_style(palette.border_variant))
        .style(background_style(palette.surface_background));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let progress_label = format!("Step {} of {}", setup.logical_step(), setup.total_steps());
    let dot_count = setup.total_steps().saturating_mul(2).saturating_sub(1);
    let progress_width = progress_label
        .width()
        .saturating_add(dot_count)
        .saturating_add(2);

    let sections = Layout::horizontal([
        Constraint::Min(12),
        Constraint::Length(progress_width.min(inner.width as usize) as u16),
    ])
    .split(inner);

    let brand = Paragraph::new(Line::from(vec![Span::styled(
        " locus.codes",
        text_style(palette.text).add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(brand, sections[0]);

    let progress = Paragraph::new(Line::from(progress_line(setup, palette)))
        .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(progress, sections[1]);
}

fn draw_footer(frame: &mut Frame, area: Rect, step: SetupStep, palette: &LocusPalette) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(border_style(palette.border_variant))
        .style(background_style(palette.surface_background));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let footer = Paragraph::new(footer_hints(step, palette));
    frame.render_widget(footer, inner);
}

fn draw_content(frame: &mut Frame, area: Rect, state: &TuiState, palette: &LocusPalette) {
    frame.render_widget(
        Block::default().style(background_style(palette.background)),
        area,
    );

    let padded = inset(area, 4, 1);
    match state.setup.step {
        SetupStep::Welcome => draw_welcome(frame, padded, palette),
        SetupStep::SelectProvider => draw_provider_selection(frame, padded, state, palette),
        SetupStep::EnterApiKey => draw_api_key(frame, padded, state, palette),
        SetupStep::LocusGraphChoice => draw_graph_choice(frame, padded, state, palette),
        SetupStep::LocusGraphUrl => draw_graph_input(
            frame,
            padded,
            state,
            palette,
            "LocusGraph server URL",
            "The gRPC endpoint for your LocusGraph instance.",
            " LOCUSGRAPH_SERVER_URL ",
            &state.setup.graph_url,
            false,
        ),
        SetupStep::LocusGraphSecret => draw_graph_input(
            frame,
            padded,
            state,
            palette,
            "LocusGraph secret",
            "Your agent authentication secret.",
            " LOCUSGRAPH_AGENT_SECRET ",
            &state.setup.graph_secret,
            true,
        ),
        SetupStep::LocusGraphId => draw_graph_input(
            frame,
            padded,
            state,
            palette,
            "Graph ID",
            "Namespace for this agent's memory, for example locus-agent.",
            " LOCUSGRAPH_GRAPH_ID ",
            &state.setup.graph_id,
            false,
        ),
        SetupStep::Confirm => draw_confirm(frame, padded, state, palette),
        SetupStep::Done => draw_done(frame, padded, state, palette),
    }
}

fn draw_welcome(frame: &mut Frame, area: Rect, palette: &LocusPalette) {
    let sections = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Min(2),
    ])
    .split(area);

    let logo = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  ▐█▌  ", text_style(palette.accent)),
            Span::styled(
                "locus.codes",
                text_style(palette.text).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            "       Terminal-native coding agent with memory",
            text_muted_style(palette.text_muted),
        )]),
    ]);
    frame.render_widget(logo, sections[0]);

    let copy = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "This wizard sets up the minimum configuration to get started.",
            text_style(palette.text),
        )]),
        Line::from(vec![Span::styled(
            "You will need an API key for at least one LLM provider.",
            text_style(palette.text),
        )]),
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(copy, sections[2]);

    let action = Paragraph::new(Line::from(vec![Span::styled(
        "Press Enter to begin ->",
        text_style(palette.accent).add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(action, sections[3]);
}

fn draw_provider_selection(
    frame: &mut Frame,
    area: Rect,
    state: &TuiState,
    palette: &LocusPalette,
) {
    let provider_count = SetupProvider::ALL.len() as u16;
    // Each provider takes 1 line + 1 blank spacer (except last), plus 1 top padding
    let list_height = provider_count * 2;
    let sections = content_sections(area, state.setup.error_message.is_some(), list_height);
    frame.render_widget(
        Paragraph::new(step_header_lines(
            "Choose your LLM provider",
            "Which provider's models should the agent use?",
            palette,
        ))
        .wrap(Wrap { trim: false }),
        sections.header,
    );

    let row_width = sections.body.width.saturating_sub(2) as usize;
    let rows: Vec<Line<'static>> = SetupProvider::ALL
        .iter()
        .enumerate()
        .flat_map(|(index, provider)| {
            [
                selection_item(
                    provider.label(),
                    provider.description(),
                    state.setup.provider_cursor == index,
                    palette,
                    row_width,
                ),
                Line::from(""),
            ]
        })
        .collect();
    frame.render_widget(
        Paragraph::new(rows).wrap(Wrap { trim: false }),
        sections.body,
    );

    if let Some(error) = &state.setup.error_message {
        draw_error(frame, sections.error, error, palette);
    }
}

fn draw_api_key(frame: &mut Frame, area: Rect, state: &TuiState, palette: &LocusPalette) {
    let sections = content_sections(area, state.setup.error_message.is_some(), 3);
    let provider = state.setup.selected_or_cursor_provider();
    frame.render_widget(
        Paragraph::new(step_header_lines(
            &format!("Enter your {} API key", provider.label()),
            "Paste your key. It is stored locally in ~/.locus.",
            palette,
        ))
        .wrap(Wrap { trim: false }),
        sections.header,
    );
    draw_input_field(
        frame,
        sections.body,
        provider.env_var(),
        &state.setup.api_key,
        true,
        state.frame_count,
        palette,
    );
    if let Some(error) = &state.setup.error_message {
        draw_error(frame, sections.error, error, palette);
    }
}

fn draw_graph_choice(frame: &mut Frame, area: Rect, state: &TuiState, palette: &LocusPalette) {
    let sections = content_sections(area, state.setup.error_message.is_some(), 4);
    frame.render_widget(
        Paragraph::new(step_header_lines(
            "Memory",
            "LocusGraph gives the agent memory across sessions.",
            palette,
        ))
        .wrap(Wrap { trim: false }),
        sections.header,
    );

    let row_width = sections.body.width.saturating_sub(2) as usize;
    let rows = vec![
        selection_item(
            "Configure now",
            "Save URL, secret, and graph ID now.",
            state.setup.graph_choice_cursor == 0,
            palette,
            row_width,
        ),
        Line::from(""),
        selection_item(
            "Skip for now",
            "Start chatting without persistent memory.",
            state.setup.graph_choice_cursor == 1,
            palette,
            row_width,
        ),
    ];
    frame.render_widget(
        Paragraph::new(rows).wrap(Wrap { trim: false }),
        sections.body,
    );

    if let Some(error) = &state.setup.error_message {
        draw_error(frame, sections.error, error, palette);
    }
}

fn draw_graph_input(
    frame: &mut Frame,
    area: Rect,
    state: &TuiState,
    palette: &LocusPalette,
    title: &str,
    description: &str,
    field_label: &str,
    value: &str,
    is_secret: bool,
) {
    let sections = content_sections(area, state.setup.error_message.is_some(), 3);
    frame.render_widget(
        Paragraph::new(step_header_lines(title, description, palette)).wrap(Wrap { trim: false }),
        sections.header,
    );
    draw_input_field(
        frame,
        sections.body,
        field_label,
        value,
        is_secret,
        state.frame_count,
        palette,
    );
    if let Some(error) = &state.setup.error_message {
        draw_error(frame, sections.error, error, palette);
    }
}

fn draw_confirm(frame: &mut Frame, area: Rect, state: &TuiState, palette: &LocusPalette) {
    let sections = content_sections(area, state.setup.error_message.is_some(), 8);
    frame.render_widget(
        Paragraph::new(step_header_lines(
            "Review your configuration",
            "Everything looks good?",
            palette,
        ))
        .wrap(Wrap { trim: false }),
        sections.header,
    );

    let provider = state.setup.selected_or_cursor_provider();
    let card = Block::default()
        .title(" Configuration ")
        .borders(Borders::ALL)
        .border_style(border_style(palette.border_variant))
        .style(background_style(palette.elevated_surface_background));
    let inner = card.inner(sections.body);
    frame.render_widget(card, sections.body);

    let mut lines = vec![
        summary_line("Provider", provider.label(), palette),
        summary_line("API Key", &mask_for_summary(&state.setup.api_key), palette),
    ];
    if state.setup.configure_graph {
        lines.push(Line::from(vec![
            Span::styled("LocusGraph   ", text_muted_style(palette.text_muted)),
            Span::styled(
                "✓ ",
                success_style(palette.success).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Configured", text_style(palette.text)),
        ]));
        lines.push(summary_line("URL", &state.setup.graph_url, palette));
        lines.push(summary_line("Graph ID", &state.setup.graph_id, palette));
    } else {
        lines.push(summary_line("LocusGraph", "Skipped", palette));
    }

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        inset(inner, 1, 1),
    );

    if let Some(error) = &state.setup.error_message {
        draw_error(frame, sections.error, error, palette);
    }
}

fn draw_done(frame: &mut Frame, area: Rect, state: &TuiState, palette: &LocusPalette) {
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(1),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "✓",
            success_style(palette.success).add_modifier(Modifier::BOLD),
        )]))
        .alignment(ratatui::layout::Alignment::Center),
        sections[0],
    );

    let shimmer_line = if let Some(shimmer) = &state.setup.done_shimmer {
        Line::from(shimmer.styled_spans_with_palette("Configuration saved.", palette))
    } else {
        Line::from(vec![Span::styled(
            "Configuration saved.",
            success_style(palette.success).add_modifier(Modifier::BOLD),
        )])
    };
    frame.render_widget(
        Paragraph::new(shimmer_line).alignment(ratatui::layout::Alignment::Center),
        sections[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "Press Enter to start chatting.",
            text_style(palette.text),
        )]))
        .alignment(ratatui::layout::Alignment::Center),
        sections[2],
    );
}

fn draw_input_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    is_secret: bool,
    frame_count: u64,
    palette: &LocusPalette,
) {
    let block = Block::default()
        .title(label.to_string())
        .borders(Borders::ALL)
        .border_style(border_focused_style(palette.border_focused))
        .style(background_style(palette.elevated_surface_background));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let display = if is_secret {
        masked_secret(value)
    } else {
        value.to_string()
    };
    let mut spans = vec![
        Span::styled(" ".to_string(), text_style(palette.text)),
        Span::styled(display, text_style(palette.text)),
    ];
    if (frame_count / 5).is_multiple_of(2) {
        spans.push(Span::styled("▌".to_string(), text_style(palette.accent)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), inner);
}

fn draw_error(frame: &mut Frame, area: Rect, error: &str, palette: &LocusPalette) {
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            error.to_string(),
            danger_style(palette.danger),
        )]))
        .wrap(Wrap { trim: false }),
        area,
    );
}

fn progress_line(setup: &SetupState, palette: &LocusPalette) -> Vec<Span<'static>> {
    let mut spans = vec![Span::styled(
        format!("Step {} of {}  ", setup.logical_step(), setup.total_steps()),
        text_muted_style(palette.text_muted),
    )];
    let current = setup.logical_step();
    let total = setup.total_steps();
    for index in 1..=total {
        let span = if setup.step == SetupStep::Done || index < current {
            Span::styled("●", text_style(palette.accent))
        } else if index == current {
            Span::styled("◉", text_style(palette.accent))
        } else {
            Span::styled("○", text_muted_style(palette.text_muted))
        };
        spans.push(span);
        if index != total {
            spans.push(Span::raw(" "));
        }
    }
    spans
}

fn footer_hints(step: SetupStep, palette: &LocusPalette) -> Line<'static> {
    let hints: &[(&str, &str)] = match step {
        SetupStep::Welcome => &[("Enter", "begin")],
        SetupStep::SelectProvider => &[("↑↓", "select"), ("Enter", "confirm")],
        SetupStep::EnterApiKey => &[("Enter", "continue"), ("Esc", "back")],
        SetupStep::LocusGraphChoice => &[("↑↓", "select"), ("Enter", "confirm"), ("Esc", "back")],
        SetupStep::LocusGraphUrl | SetupStep::LocusGraphSecret | SetupStep::LocusGraphId => {
            &[("Enter", "continue"), ("Esc", "back")]
        }
        SetupStep::Confirm => &[("Enter", "save & start"), ("Esc", "back")],
        SetupStep::Done => &[("Enter", "start chatting")],
    };

    let mut spans = vec![Span::raw(" ")];
    for (index, (key, action)) in hints.iter().enumerate() {
        spans.push(Span::styled(
            key.to_string(),
            text_style(palette.accent).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            action.to_string(),
            text_muted_style(palette.text_muted),
        ));
        if index + 1 != hints.len() {
            spans.push(Span::styled(
                "  │  ".to_string(),
                border_style(palette.border_variant),
            ));
        }
    }
    Line::from(spans)
}

fn step_header_lines(title: &str, description: &str, palette: &LocusPalette) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            title.to_string(),
            text_style(palette.text).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            description.to_string(),
            text_muted_style(palette.text_muted),
        )]),
    ]
}

fn selection_item(
    label: &str,
    description: &str,
    selected: bool,
    palette: &LocusPalette,
    width: usize,
) -> Line<'static> {
    let prefix = if selected { "› " } else { "  " };
    let content = format!("{}{label}  {description}", prefix);
    let padded = format!("{content:<width$}");
    let style = if selected {
        text_style(palette.text).bg(rgb_to_color(palette.element_selected))
    } else {
        text_style(palette.text)
    };
    let prefix_style = if selected {
        text_style(palette.accent)
            .bg(rgb_to_color(palette.element_selected))
            .add_modifier(Modifier::BOLD)
    } else {
        text_muted_style(palette.text_muted)
    };

    let split_at = (prefix.len() + label.len()).min(padded.len());
    let (lead, rest) = padded.split_at(split_at);
    Line::from(vec![
        Span::styled(lead.to_string(), prefix_style),
        Span::styled(rest.to_string(), style),
    ])
}

fn summary_line(label: &str, value: &str, palette: &LocusPalette) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<12}"), text_muted_style(palette.text_muted)),
        Span::styled(value.to_string(), text_style(palette.text)),
    ])
}

fn masked_secret(value: &str) -> String {
    "*".repeat(value.chars().count())
}

fn mask_for_summary(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        return "*".repeat(chars.len());
    }
    format!(
        "{}...{}",
        chars.iter().take(4).collect::<String>(),
        chars.iter().skip(chars.len() - 4).collect::<String>()
    )
}

fn centered_rect(area: Rect) -> Rect {
    let width = area.width.saturating_sub(2).min(78).max(20);
    let height = area.height.saturating_sub(2).min(24).max(12);
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.max(1), height.max(1))
}

fn inset(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    let x = area.x.saturating_add(horizontal);
    let y = area.y.saturating_add(vertical);
    let width = area.width.saturating_sub(horizontal.saturating_mul(2));
    let height = area.height.saturating_sub(vertical.saturating_mul(2));
    Rect::new(x, y, width.max(1), height.max(1))
}

struct ContentSections {
    header: Rect,
    body: Rect,
    error: Rect,
}

fn content_sections(area: Rect, has_error: bool, body_height: u16) -> ContentSections {
    let error_height = if has_error { 1 } else { 0 };
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(body_height),
        Constraint::Length(error_height),
        Constraint::Min(0),
    ])
    .split(area);
    ContentSections {
        header: sections[0],
        body: sections[1],
        error: sections[2],
    }
}
