//! Spinner: animated loading indicator.

use locus_constant::theme::dark;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::time::Instant;

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const INTERVAL_MS: u64 = 80;

/// Inline spinner (single character animation).
#[derive(Debug)]
pub struct Spinner {
    start: Instant,
}

impl Default for Spinner {
    fn default() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Spinner {
    pub fn new() -> Self {
        Self::default()
    }

    fn frame_index(&self) -> usize {
        let elapsed = self.start.elapsed().as_millis() as u64;
        ((elapsed / INTERVAL_MS) as usize) % FRAMES.len()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let primary = Color::Rgb(dark::PRIMARY.0, dark::PRIMARY.1, dark::PRIMARY.2);
        let idx = self.frame_index();
        let line = Line::from(FRAMES[idx]).style(Style::default().fg(primary));
        let p = Paragraph::new(line);
        frame.render_widget(p, area);
    }
}
