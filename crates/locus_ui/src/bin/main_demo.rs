//! Demo binary: run the main view with sample content.
//! Press `q` or Ctrl+C to quit, `t` to toggle theme, `p` to toggle panel.

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
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
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new();

    // Add sample messages
    state.chat.push(Message::user("Hello! Can you help me with my Rust project?"));
    state.chat.push(Message::assistant_text("Of course! I'd be happy to help with your Rust project. What would you like to work on?"));
    state.chat.push(Message::user("I need to add error handling to my file parser."));
    state.chat.push(Message::assistant_text("I'll help you add proper error handling. Let me first check your current implementation to understand the context."));
    
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
            if let Event::Key(e) = event::read()? {
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
                    KeyCode::Char(ch) => state.input.insert(ch),
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
