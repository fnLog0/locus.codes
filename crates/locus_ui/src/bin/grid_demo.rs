//! Demo binary: run the Grid component with borders and cell labels.
//! Press `q` or Ctrl+C to quit.

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use locus_ui::Grid;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

fn inner_rect(r: Rect) -> Rect {
    if r.width < 2 || r.height < 2 {
        return r;
    }
    Rect {
        x: r.x + 1,
        y: r.y + 1,
        width: r.width - 2,
        height: r.height - 2,
    }
}
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

    let grid = Grid::new(3, 4).with_gap(1, 1).with_borders(true);

    loop {
        if QUIT.load(Ordering::Relaxed) {
            break;
        }

        terminal.draw(|f| {
            let area = f.area();
            grid.render(f, area);

            let cells = grid.cell_rects(area);
            for (row, row_rects) in cells.iter().enumerate() {
                for (col, rect) in row_rects.iter().enumerate() {
                    let label = format!("{},{}", row, col);
                    let para = Paragraph::new(Line::from(label))
                        .alignment(Alignment::Center)
                        .style(Style::default());
                    f.render_widget(para, inner_rect(*rect));
                }
            }
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
