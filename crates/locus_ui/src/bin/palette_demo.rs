//! Color palette demo for light and dark theme.
//! Press `t` to switch theme, `q` or Ctrl+C to quit.

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use locus_ui::theme::{Theme, ThemeMode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Terminal;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static QUIT: AtomicBool = AtomicBool::new(false);

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(|| QUIT.store(true, Ordering::Relaxed))?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut theme = Theme::dark();

    loop {
        if QUIT.load(Ordering::Relaxed) {
            break;
        }

        terminal.draw(|f| draw_palette(f, &theme))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(e) = event::read()? {
                if e.kind != KeyEventKind::Press {
                    continue;
                }
                match e.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char('c') if e.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('t') | KeyCode::Char('T') => theme.toggle(),
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn draw_palette(f: &mut Frame, theme: &Theme) {
    let area = f.area();
    let mode_label = match theme.mode() {
        ThemeMode::Dark => "Dark",
        ThemeMode::Light => "Light",
    };

    let bg_block = Block::default().style(Style::default().bg(theme.bg));
    f.render_widget(bg_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Line::from(vec![
        Span::styled(" Color palette ", Style::default().fg(theme.primary_fg).bg(theme.primary)),
        Span::raw(" "),
        Span::styled(mode_label, Style::default().fg(theme.accent)),
        Span::styled(" — press ", Style::default().fg(theme.muted_fg)),
        Span::styled("t", Style::default().fg(theme.primary).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" to switch theme, ", Style::default().fg(theme.muted_fg)),
        Span::styled("q", Style::default().fg(theme.primary).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" to quit", Style::default().fg(theme.muted_fg)),
    ]);
    f.render_widget(Paragraph::new(title), chunks[0]);

    let colors: &[(&str, ratatui::style::Color)] = &[
        ("background", theme.bg),
        ("foreground", theme.fg),
        ("card", theme.card),
        ("primary", theme.primary),
        ("primary_fg", theme.primary_fg),
        ("secondary", theme.secondary),
        ("muted", theme.muted),
        ("muted_fg", theme.muted_fg),
        ("faint", theme.faint),
        ("accent", theme.accent),
        ("danger", theme.danger),
        ("success", theme.success),
        ("warning", theme.warning),
        ("info", theme.info),
        ("purple", theme.purple),
        ("border", theme.border),
        ("input", theme.input),
        ("ring", theme.ring),
        ("code_bg", theme.code_bg),
        ("tool_bg", theme.tool_bg),
        ("bash_bg", theme.bash_bg),
        ("think_bg", theme.think_bg),
        ("file_path", theme.file_path),
        ("tool_name", theme.tool_name),
        ("timestamp", theme.timestamp),
    ];

    let body = chunks[1];
    let (left, right) = split_horizontal(body);

    let mut left_lines: Vec<Line> = Vec::new();
    let mut right_lines: Vec<Line> = Vec::new();
    let swatch_w = 6;

    for (i, (name, color)) in colors.iter().enumerate() {
        let line = Line::from(vec![
            Span::styled(" ".repeat(swatch_w), Style::default().bg(*color)),
            Span::raw(" "),
            Span::styled(*name, Style::default().fg(theme.fg)),
        ]);
        if i < (colors.len() + 1) / 2 {
            left_lines.push(line);
        } else {
            right_lines.push(line);
        }
    }

    let left_para = Paragraph::new(left_lines).style(Style::default().bg(theme.bg));
    let right_para = Paragraph::new(right_lines).style(Style::default().bg(theme.bg));
    f.render_widget(left_para, left);
    f.render_widget(right_para, right);

    let help = Line::from(Span::styled(
        " t: switch theme   q: quit ",
        Style::default().fg(theme.muted_fg),
    ));
    f.render_widget(Paragraph::new(help), chunks[2]);
}

fn split_horizontal(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    (chunks[0], chunks[1])
}
