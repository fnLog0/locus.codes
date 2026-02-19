//! Demo binary: full chat layout with header, messages, input, and shortcuts.
//! Press Enter to send, Ctrl+C or q to quit, Ctrl+L to toggle theme.

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use locus_ui::{Chat, Header, Input, Message, ShortcutsBar, Theme};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static QUIT: AtomicBool = AtomicBool::new(false);

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(|| QUIT.store(true, Ordering::Relaxed))?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut theme = Theme::default();
    let mut header = Header::new();
    let mut chat = Chat::new();
    let mut input = Input::new();

    // Initialize header with current directory
    header.update_directory(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    header.update_branch(detect_git_branch());

    // Add some demo messages
    chat.push(Message::user("Hello! Can you help me with Rust?"));
    chat.push(Message::assistant_text(
        "Of course! I'd be happy to help with Rust. What would you like to know?",
    ));
    chat.push(Message::user("How do I implement a trait?"));
    chat.push(Message::assistant_text(
        "To implement a trait in Rust, you use the `impl` keyword. Here's a basic example:\n\n    trait Greet {\n        fn greet(&self) -> String;\n    }\n    \n    struct Person { name: String }\n    \n    impl Greet for Person {\n        fn greet(&self) -> String {\n            format!(\"Hello, {}!\", self.name)\n        }\n    }",
    ));
    chat.push(Message::user("Thanks! That's helpful."));
    chat.push(Message::assistant_text(
        "You're welcome! Let me know if you have any other questions about Rust.",
    ));

    loop {
        if QUIT.load(Ordering::Relaxed) {
            break;
        }

        header.update_time();

        terminal.draw(|f| {
            let area = f.area();
            let theme_ref = &theme;

            // Layout: header (1), chat (flex), input (1), shortcuts (1)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([
                    Constraint::Length(1), // Header
                    Constraint::Min(1),    // Chat
                    Constraint::Length(1), // Input
                    Constraint::Length(1), // Shortcuts
                ])
                .split(area);

            // Render each component
            header.render(f, chunks[0], theme_ref);
            chat.render(f, chunks[1], theme_ref);
            input.render(f, chunks[2], theme_ref);
            shortcuts_bar().render(f, chunks[3], theme_ref);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let evt = event::read()?;
            match evt {
                Event::Key(e) => {
                    if e.kind != KeyEventKind::Press {
                        continue;
                    }

                    match (e.code, e.modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Char('q'), _) => {
                            break;
                        }
                        (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                            theme.toggle();
                        }
                        (KeyCode::Up, KeyModifiers::NONE) => {
                            chat.scroll_up(1);
                        }
                        (KeyCode::Down, KeyModifiers::NONE) => {
                            chat.scroll_down(1);
                        }
                        (KeyCode::PageUp, _) => {
                            chat.page_up();
                        }
                        (KeyCode::PageDown, _) => {
                            chat.page_down();
                        }
                        (KeyCode::Enter, _) => {
                            if let Some(text) = input.submit() {
                                chat.push(Message::user(&text));
                                // Simulate assistant response
                                chat.push(Message::assistant_text(format!(
                                    "You said: \"{}\"",
                                    text
                                )));
                                // Scroll to bottom after sending
                                chat.scroll_to_bottom();
                            }
                        }
                        (KeyCode::Backspace, _) => {
                            input.backspace();
                        }
                        (KeyCode::Delete, _) => {
                            input.delete();
                        }
                        (KeyCode::Left, _) => {
                            input.move_left();
                        }
                        (KeyCode::Right, _) => {
                            input.move_right();
                        }
                        (KeyCode::Home, _) => {
                            input.move_home();
                        }
                        (KeyCode::End, _) => {
                            input.move_end();
                        }
                        (KeyCode::Char(ch), KeyModifiers::NONE) => {
                            input.insert(ch);
                        }
                        _ => {}
                    }
                }
                Event::Mouse(MouseEvent { kind, .. }) => match kind {
                    MouseEventKind::ScrollUp => {
                        chat.scroll_up(3);
                    }
                    MouseEventKind::ScrollDown => {
                        chat.scroll_down(3);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn shortcuts_bar() -> ShortcutsBar {
    ShortcutsBar::new()
}

fn detect_git_branch() -> Option<String> {
    // Try to read .git/HEAD
    let head_path = std::path::Path::new(".git/HEAD");
    if head_path.exists() {
        if let Ok(content) = std::fs::read_to_string(head_path) {
            // Format: "ref: refs/heads/branch-name" or just a commit hash
            if content.starts_with("ref: refs/heads/") {
                return content
                    .strip_prefix("ref: refs/heads/")
                    .map(|s| s.trim().to_string());
            } else {
                // Detached HEAD - show first 7 chars of commit
                return content.trim().get(..7).map(|s| s.to_string());
            }
        }
    }
    None
}
