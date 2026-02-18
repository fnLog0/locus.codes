//! Demo binary: run the loading screen with shimmer and progress bar.
//! Press `q` or Ctrl+C to quit.

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use locus_ui::Loader;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static QUIT: AtomicBool = AtomicBool::new(false);

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(|| QUIT.store(true, Ordering::Relaxed))?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut loader = Loader::new().with_footer_message("Initializing…");
    let start = Instant::now();

    loop {
        if QUIT.load(Ordering::Relaxed) {
            break;
        }

        let progress = (start.elapsed().as_secs_f64() * 0.15).min(0.95);
        loader.set_progress(progress);
        terminal.draw(|f| loader.render(f, f.area()))?;

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
