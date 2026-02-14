//! locus-ui — ratatui + crossterm TUI (plan §0.3).
//!
//! Reuses UI building blocks from services: textarea, wrapping, editor, layout.

mod detect_term;
mod editor;
mod layout;
mod markdown_renderer;
mod placeholder_prompts;
mod popup;
mod syntax_highlighter;
mod textarea;
mod views;
mod wrapping;

pub use detect_term::{detect_terminal, should_use_rgb_colors, AdaptiveColors};
pub use editor::{detect_editor, open_in_editor};
pub use layout::centered_rect;
pub use markdown_renderer::render_markdown_to_lines_with_width;
pub use placeholder_prompts::get_placeholder_prompt;
pub use popup::{NavigationPopup, PopupMode};
pub use syntax_highlighter::apply_syntax_highlighting;
pub use textarea::{TextArea, TextAreaState};

use locus_core::{RuntimeEvent, SessionState};
use std::sync::mpsc::Receiver;

/// View types for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    TaskBoard,
    Plan,
    Agents,
    DiffReview,
    Logs,
    MemoryTrace,
}

impl View {
    pub fn name(&self) -> &'static str {
        match self {
            View::TaskBoard => "Task Board",
            View::Plan => "Plan",
            View::Agents => "Agents",
            View::DiffReview => "Diff Review",
            View::Logs => "Logs",
            View::MemoryTrace => "Memory Trace",
        }
    }
}

/// View router - stack-based navigation
#[derive(Debug, Default)]
pub struct ViewRouter {
    stack: Vec<View>,
}

impl ViewRouter {
    pub fn new() -> Self {
        Self {
            stack: vec![View::TaskBoard],
        }
    }

    /// Get current view (top of stack)
    pub fn current(&self) -> View {
        *self.stack.last().unwrap_or(&View::TaskBoard)
    }

    /// Push a view onto the stack
    pub fn push(&mut self, view: View) {
        self.stack.push(view);
    }

    /// Replace the current view (don't push, just replace top)
    pub fn replace(&mut self, view: View) {
        if let Some(top) = self.stack.last_mut() {
            *top = view;
        } else {
            self.stack.push(view);
        }
    }

    /// Pop the current view (go back)
    pub fn pop(&mut self) -> Option<View> {
        if self.stack.len() > 1 {
            self.stack.pop()
        } else {
            None // Always keep at least TaskBoard
        }
    }

    /// Switch directly to a view (replace top)
    pub fn switch(&mut self, view: View) {
        self.replace(view);
    }
}

/// Command palette state
#[derive(Debug, Default)]
struct CommandPalette {
    active: bool,
    input: String,
}

impl CommandPalette {
    fn new() -> Self {
        Self::default()
    }

    fn activate(&mut self) {
        self.active = true;
        self.input = ":".to_string();
    }

    fn deactivate(&mut self) {
        self.active = false;
        self.input.clear();
    }

    fn is_active(&self) -> bool {
        self.active
    }
}

/// Run the TUI: nav bar, main content (task board), prompt bar.
/// Blocks until user quits. Sends prompts via `prompt_tx`; receives events via `event_rx`.
/// Prompt bar uses the full textarea from services (cursor, wrap, Emacs-style keys).
pub fn run_ui(
    session: SessionState,
    _event_tx: locus_core::EventTx,
    event_rx: Receiver<RuntimeEvent>,
    prompt_tx: tokio::sync::mpsc::Sender<String>,
) -> anyhow::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let mut terminal =
        ratatui::prelude::Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;

    let mut text_area = TextArea::new();
    text_area.set_placeholder(Some(" Type a prompt... "));
    let mut prompt_state = TextAreaState::default();
    let mut last_events: Vec<RuntimeEvent> = Vec::new();
    let mut view_router = ViewRouter::new();
    let mut command_palette = CommandPalette::new();
    let mut nav_popup = NavigationPopup::new();

    loop {
        while let Ok(ev) = event_rx.try_recv() {
            last_events.push(ev);
            if last_events.len() > 100 {
                last_events.remove(0);
            }
        }

        terminal.draw(|f| {
            let chunks = ratatui::prelude::Layout::default()
                .direction(ratatui::prelude::Direction::Vertical)
                .constraints([
                    ratatui::prelude::Constraint::Length(1),
                    ratatui::prelude::Constraint::Min(1),
                    ratatui::prelude::Constraint::Length(3),
                ])
                .split(f.area());

            let nav = format!(
                " locus.codes  [mode: {}]  [view: {}] ",
                session.mode,
                view_router.current().name()
            );
            f.render_widget(
                ratatui::widgets::Paragraph::new(nav)
                    .style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan))
                    .block(
                        ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .border_style(
                                ratatui::style::Style::default()
                                    .fg(crate::detect_term::AdaptiveColors::green()),
                            ),
                    ),
                chunks[0],
            );

            // Render current view
            let content_block = ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(
                    ratatui::style::Style::default().fg(crate::detect_term::AdaptiveColors::blue()),
                )
                .title(format!(" {} ", view_router.current().name()));
            let content_inner = content_block.inner(chunks[1]);
            f.render_widget(content_block, chunks[1]);

            match view_router.current() {
                View::TaskBoard => {
                    views::render_task_board_view(f, content_inner, &last_events);
                }
                View::Plan => {
                    views::render_plan_view(f, content_inner);
                }
                View::Agents => {
                    views::render_agents_view(f, content_inner);
                }
                View::DiffReview => {
                    views::render_diff_review_view(f, content_inner);
                }
                View::Logs => {
                    views::render_logs_view(f, content_inner);
                }
                View::MemoryTrace => {
                    views::render_memory_trace_view(f, content_inner);
                }
            }

            let prompt_block = if command_palette.is_active() {
                ratatui::widgets::Block::default()
                    .title(" Command ")
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(
                        ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
                    )
            } else {
                ratatui::widgets::Block::default()
                    .title(" Prompt (? for shortcuts) ")
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(
                        ratatui::style::Style::default()
                            .fg(crate::detect_term::AdaptiveColors::orange()),
                    )
            };
            let prompt_inner = prompt_block.inner(chunks[2]);

            if command_palette.is_active() {
                // Render command input directly
                f.render_widget(
                    ratatui::widgets::Paragraph::new(command_palette.input.as_str()),
                    prompt_inner,
                );
            } else {
                text_area.render_with_state(prompt_inner, f.buffer_mut(), &mut prompt_state, false);
            }
            f.render_widget(prompt_block, chunks[2]);

            // Render navigation popup if active
            if nav_popup.is_active() {
                nav_popup.render(f, f.area());
            }
        })?;

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(k) => {
                    // Command palette handling
                    if command_palette.is_active() {
                        match k.code {
                            crossterm::event::KeyCode::Enter => {
                                let cmd = command_palette.input.trim_start_matches(':').trim();
                                // Parse and execute command
                                if cmd.starts_with("mode ") {
                                    let mode_str = cmd.strip_prefix("mode ").unwrap_or("");
                                    // Send as special command to orchestrator
                                    let _ = prompt_tx.blocking_send(format!(":mode {}", mode_str));
                                } else if cmd.starts_with("view ") {
                                    let view_str =
                                        cmd.strip_prefix("view ").unwrap_or("").to_lowercase();
                                    match view_str.as_str() {
                                        "taskboard" | "task" | "1" => {
                                            view_router.switch(View::TaskBoard)
                                        }
                                        "plan" | "2" => view_router.switch(View::Plan),
                                        "agents" | "3" => view_router.switch(View::Agents),
                                        "diff" | "diffreview" | "4" => {
                                            view_router.switch(View::DiffReview)
                                        }
                                        "logs" | "5" => view_router.switch(View::Logs),
                                        "memory" | "memorytrace" | "6" => {
                                            view_router.switch(View::MemoryTrace)
                                        }
                                        _ => {}
                                    }
                                } else if cmd == "quit" || cmd == "q" {
                                    break;
                                } else if cmd == "cancel" {
                                    // Send cancel command to orchestrator
                                    let _ = prompt_tx.blocking_send(":cancel".to_string());
                                }
                                command_palette.deactivate();
                                continue;
                            }
                            crossterm::event::KeyCode::Esc => {
                                command_palette.deactivate();
                                continue;
                            }
                            crossterm::event::KeyCode::Char(c) => {
                                command_palette.input.push(c);
                                continue;
                            }
                            crossterm::event::KeyCode::Backspace => {
                                command_palette.input.pop();
                                continue;
                            }
                            _ => continue,
                        }
                    }

                    // Navigation popup handling
                    if nav_popup.is_active() {
                        match k.code {
                            crossterm::event::KeyCode::Esc => {
                                nav_popup.deactivate();
                                continue;
                            }
                            crossterm::event::KeyCode::Char(c) => {
                                nav_popup.handle_input(c);
                                continue;
                            }
                            crossterm::event::KeyCode::Backspace => {
                                nav_popup.handle_backspace();
                                continue;
                            }
                            _ => continue, // Ignore all other keys when popup is active
                        }
                    }

                    // Activate command palette with ':'
                    if k.code == crossterm::event::KeyCode::Char(':') {
                        command_palette.activate();
                        continue;
                    }

                    // Activate navigation popup with '?'
                    if k.code == crossterm::event::KeyCode::Char('?') {
                        nav_popup.activate();
                        continue;
                    }

                    // Track if key was handled by global shortcuts
                    let mut key_handled = false;

                    // View switching (1-6)
                    match k.code {
                        crossterm::event::KeyCode::Char('1') => {
                            view_router.switch(View::TaskBoard);
                            key_handled = true;
                        }
                        crossterm::event::KeyCode::Char('2') => {
                            view_router.switch(View::Plan);
                            key_handled = true;
                        }
                        crossterm::event::KeyCode::Char('3') => {
                            view_router.switch(View::Agents);
                            key_handled = true;
                        }
                        crossterm::event::KeyCode::Char('4') => {
                            view_router.switch(View::DiffReview);
                            key_handled = true;
                        }
                        crossterm::event::KeyCode::Char('5') => {
                            view_router.switch(View::Logs);
                            key_handled = true;
                        }
                        crossterm::event::KeyCode::Char('6') => {
                            view_router.switch(View::MemoryTrace);
                            key_handled = true;
                        }
                        crossterm::event::KeyCode::F(1) => {
                            // Switch to Rush mode (send command to orchestrator)
                            let _ = prompt_tx.blocking_send(":mode rush".to_string());
                            key_handled = true;
                        }
                        crossterm::event::KeyCode::F(2) => {
                            // Switch to Smart mode
                            let _ = prompt_tx.blocking_send(":mode smart".to_string());
                            key_handled = true;
                        }
                        crossterm::event::KeyCode::F(3) => {
                            // Switch to Deep mode
                            let _ = prompt_tx.blocking_send(":mode deep".to_string());
                            key_handled = true;
                        }
                        _ => {}
                    }

                    // Global shortcuts
                    if k.code == crossterm::event::KeyCode::Char('q')
                        && k.modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        break;
                    }
                    if k.code == crossterm::event::KeyCode::Esc {
                        view_router.pop();
                        key_handled = true;
                    }

                    // Navigation keybindings (j/k, g/G, /)
                    // TODO: Implement proper scrolling in views

                    if k.code == crossterm::event::KeyCode::Enter {
                        let prompt = text_area.text().trim().to_string();
                        text_area.set_text("");
                        if !prompt.is_empty() {
                            let _ = prompt_tx.blocking_send(prompt);
                        }
                        key_handled = true;
                    }

                    // Only send unhandled keys to textarea
                    if !key_handled {
                        text_area.input(k);
                    }
                }
                _ => {}
            }
        }
    }

    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    crossterm::terminal::disable_raw_mode()?;
    terminal.show_cursor()?;
    Ok(())
}
