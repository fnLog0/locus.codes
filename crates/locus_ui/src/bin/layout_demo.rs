//! Demo binary: render the header only (left = locus.codes, right = branch • dir • time).
//! Press `q` or Ctrl+C to quit.

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use locus_ui::{Header, Theme};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

static QUIT: AtomicBool = AtomicBool::new(false);

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(|| QUIT.store(true, Ordering::Relaxed))?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = Theme::default();
    let mut header = Header::new();
    header.update_branch(Some("master".to_string()));
    header.update_directory(std::env::current_dir().unwrap_or_default());

    loop {
        if QUIT.load(Ordering::Relaxed) {
            break;
        }

        header.update_time();
        terminal.draw(|f| {
            let area = f.area();
            let header_rect = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            };
            header.render(f, header_rect, &theme);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(e) = event::read()? {
                if e.kind != KeyEventKind::Press {
                    continue;
                }
                let quit = e.code == KeyCode::Char('q')
                    || (e.code == KeyCode::Char('c') && e.modifiers.contains(KeyModifiers::CONTROL));
                if quit {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
