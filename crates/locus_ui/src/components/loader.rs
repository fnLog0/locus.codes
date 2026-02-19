//! Loading screen component: shimmer title and footer progress.

use crate::animation::Shimmer;
use super::constants::HORIZONTAL_PADDING;
use locus_constant::theme::dark;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};
use ratatui::Frame;

const TITLE_TEXT: &str = "locus.";
const FOOTER_LOADING_MSG: &str = "Initializing…";
/// Rows between title and "Initializing…".
const TITLE_TO_INITIALIZING_MARGIN: u16 = 2;
/// Rows reserved at bottom for progress bar (+ padding).
const LOADER_HEIGHT: u16 = 2;

/// Pixelated (block) title lines for "locus.".
fn big_title_lines() -> Vec<String> {
    super::pixel_font::pixel_lines(TITLE_TEXT)
}

/// Loading screen with shimmer title and footer progress.
#[derive(Debug)]
pub struct Loader {
    shimmer: Shimmer,
    /// Progress 0.0..=1.0 for the footer bar.
    progress: f64,
    /// Optional custom footer message (default: FOOTER_LOADING_MSG).
    footer_message: String,
    /// Cached big-title lines (one ASCII-art shape).
    big_title_lines: Vec<String>,
}

impl Default for Loader {
    fn default() -> Self {
        Self {
            shimmer: Shimmer::new(),
            progress: 0.0,
            footer_message: FOOTER_LOADING_MSG.to_string(),
            big_title_lines: big_title_lines(),
        }
    }
}

impl Loader {
    /// Creates a new loader.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the footer message.
    pub fn with_footer_message(mut self, msg: impl Into<String>) -> Self {
        self.footer_message = msg.into();
        self
    }

    /// Sets progress for the footer bar (0.0..=1.0).
    pub fn set_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    /// Advances the shimmer and then renders the loading screen into `frame` in `area`.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.shimmer.tick();

        let title_height = self
            .big_title_lines
            .len()
            .min(area.height as usize) as u16;
        // Region for title + margin + "Initializing…" (loader is fixed at bottom)
        let title_region_height = area
            .height
            .saturating_sub(LOADER_HEIGHT)
            .saturating_sub(1)
            .saturating_sub(TITLE_TO_INITIALIZING_MARGIN);
        let title_top = title_region_height.saturating_sub(title_height) / 2;
        let title_rect = Rect {
            x: area.x,
            y: area.y + title_top,
            width: area.width,
            height: title_height,
        };

        // locus. centered in the upper area
        let title_lines: Vec<Line> = self
            .big_title_lines
            .iter()
            .map(|s| Line::from(self.shimmer.styled_spans(s)))
            .collect();
        let title = Paragraph::new(title_lines).alignment(Alignment::Center);
        frame.render_widget(title, title_rect);

        // "Initializing…" just below title with margin
        let initializing_y = area.y + title_region_height + TITLE_TO_INITIALIZING_MARGIN;
        let muted = Color::Rgb(dark::MUTED_FG.0, dark::MUTED_FG.1, dark::MUTED_FG.2);
        let initializing = Paragraph::new(Line::from(Span::styled(
            self.footer_message.as_str(),
            Style::default().fg(muted),
        )))
        .alignment(Alignment::Center);
        let initializing_rect = Rect {
            x: area.x,
            y: initializing_y,
            width: area.width,
            height: 1,
        };
        frame.render_widget(initializing, initializing_rect);

        // Loader (progress bar) at bottom
        let gauge_y = area.y + area.height.saturating_sub(1);
        let primary = Color::Rgb(dark::PRIMARY.0, dark::PRIMARY.1, dark::PRIMARY.2);
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(primary))
            .ratio(self.progress)
            .label(Span::raw(""));
        let gauge_area = Rect {
            x: area.x + HORIZONTAL_PADDING,
            y: gauge_y,
            width: area.width.saturating_sub(HORIZONTAL_PADDING * 2),
            height: 1,
        };
        frame.render_widget(gauge, gauge_area);
    }

    /// Resets the loader (shimmer and progress).
    pub fn reset(&mut self) {
        self.shimmer.reset();
        self.progress = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_default_footer_message() {
        let loader = Loader::default();
        assert_eq!(loader.footer_message, FOOTER_LOADING_MSG);
    }
}