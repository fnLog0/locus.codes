//! Demo binary: run the main view with sample content.
//! Only the chat (messages) area scrolls; status, input, and shortcuts stay fixed.
//! Scroll: mouse wheel, or ↑/↓ (when input empty), or PgUp/PgDn. Quit: `q` or Ctrl+C.

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use locus_ui::{view, AppState, Message};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static QUIT: AtomicBool = AtomicBool::new(false);

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(|| QUIT.store(true, Ordering::Relaxed))?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new();

    // Add enough sample messages so the chat overflows and only the chat area scrolls
    state.chat.push(Message::user("Hello! Can you help me with my Rust project?"));
    state.chat.push(Message::assistant_text("Of course! I'd be happy to help with your Rust project. What would you like to work on?"));
    state.chat.push(Message::user("I need to add error handling to my file parser."));
    state.chat.push(Message::assistant_text("I'll help you add proper error handling. Let me first check your current implementation to understand the context."));
    state.chat.push(Message::user("The file is at src/parser.rs. It's about 200 lines."));
    state.chat.push(Message::assistant_text("I've opened src/parser.rs. You're using Result<T, Box<dyn Error>> in a few places. We can switch to a custom error type and use thiserror for better diagnostics. Should I sketch the error enum and the From impls?"));
    state.chat.push(Message::user("Yes, and keep the existing API if possible."));
    state.chat.push(Message::assistant_text("Here’s a minimal error enum that keeps your current API and adds better errors:\n\n#[derive(Debug, thiserror::Error)]\npub enum ParseError { ... }\n\nI'll add the impl and wire it into parse_file next."));

    // Simulate a tool use in the last message
    let mut last_msg = Message::assistant_text("");
    last_msg.content.push(locus_ui::ContentBlock::ToolUse(locus_ui::ToolDisplay {
        id: "tool_1".to_string(),
        name: "read_file".to_string(),
        args: serde_json::json!({"file_path": "src/parser.rs"}),
        status: locus_ui::ToolStatus::Done,
        output: Some("fn parse_file(path: &str) -> Result<Data, Box<dyn Error>> {\n    let content = std::fs::read_to_string(path)?;\n    // parsing logic\n}".to_string()),
        duration: Some(Duration::from_millis(42)),
    }));
    state.chat.push(last_msg);

    let mut last_tick = Instant::now();

    loop {
        if QUIT.load(Ordering::Relaxed) {
            break;
        }

        // Update time every second (for future use)
        if last_tick.elapsed() >= Duration::from_secs(1) {
            state.update_time();
            last_tick = Instant::now();
        }

        terminal.draw(|f| view(f, &mut state))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(e) => {
                    if e.kind != KeyEventKind::Press {
                        continue;
                    }
                    match e.code {
                    KeyCode::Char('c') if e.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('q') if state.input.is_empty() => break,
                    KeyCode::Enter => {
                        if let Some(text) = state.input.submit() {
                            if !text.is_empty() {
                                state.chat.push(Message::user(&text));
                            }
                        }
                    }
                    KeyCode::Backspace => state.input.backspace(),
                    KeyCode::Delete => state.input.delete(),
                    KeyCode::Left => state.input.move_left(),
                    KeyCode::Right => state.input.move_right(),
                    KeyCode::Home => state.input.move_home(),
                    KeyCode::End => state.input.move_end(),
                    KeyCode::Up => {
                        if state.input.is_empty() {
                            state.chat.scroll_up(1);
                        } else {
                            state.input.history_up();
                        }
                    }
                    KeyCode::Down => {
                        if state.input.is_empty() {
                            state.chat.scroll_down(1);
                        } else {
                            state.input.history_down();
                        }
                    }
                    KeyCode::PageUp => {
                        if state.input.is_empty() {
                            state.chat.page_up();
                        }
                    }
                    KeyCode::PageDown => {
                        if state.input.is_empty() {
                            state.chat.page_down();
                        }
                    }
                    KeyCode::Char(ch) => state.input.insert(ch),
                    _ => {}
                }
                }
                Event::Mouse(m) => {
                    // Mouse wheel: scroll chat (only the chat area scrolls)
                    let lines = 3;
                    match m.kind {
                        MouseEventKind::ScrollUp => state.chat.scroll_up(lines),
                        MouseEventKind::ScrollDown => state.chat.scroll_down(lines),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
