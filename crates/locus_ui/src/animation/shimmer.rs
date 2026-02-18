//! Shimmer animation: a moving highlight (left-to-right) over text.

use locus_constant::theme::dark;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use std::time::Instant;

/// Width of the shimmer highlight as a fraction of the text length (0.0..=1.0).
const SHIMMER_WIDTH: f64 = 0.35;

/// How much the highlight position advances per second (0.0..=1.0 per second).
const SHIMMER_SPEED: f64 = 0.4;

/// Shimmer state: position and timing for a left-to-right light reflection.
#[derive(Debug, Clone)]
pub struct Shimmer {
    /// Current position of the shimmer center (0.0 = left, 1.0 = right).
    position: f64,
    /// Last time we advanced the position.
    last_tick: Instant,
    /// Whether the animation is paused.
    paused: bool,
}

impl Default for Shimmer {
    fn default() -> Self {
        Self {
            position: 0.0,
            last_tick: Instant::now(),
            paused: false,
        }
    }
}

impl Shimmer {
    /// Creates a new shimmer animation starting at the left.
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances the shimmer position by elapsed time. Call once per frame.
    pub fn tick(&mut self) {
        if self.paused {
            return;
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_tick);
        self.last_tick = now;
        let delta = (elapsed.as_secs_f64() * SHIMMER_SPEED).min(0.1);
        self.position += delta;
        if self.position > 1.0 + SHIMMER_WIDTH {
            self.position -= 1.0 + SHIMMER_WIDTH;
        }
    }

    /// Pauses or resumes the animation.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        if !paused {
            self.last_tick = Instant::now();
        }
    }

    /// Resets the shimmer to the start.
    pub fn reset(&mut self) {
        self.position = 0.0;
        self.last_tick = Instant::now();
    }

    /// Returns styled spans for the given text with the current shimmer applied.
    /// Each character gets an intensity based on distance from the shimmer center.
    pub fn styled_spans(&self, text: &str) -> Vec<Span<'static>> {
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return vec![];
        }
        let n = chars.len() as f64;
        let half_width = SHIMMER_WIDTH / 2.0;
        let center = self.position * (1.0 + 2.0 * half_width) - half_width;

        chars
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                let char_pos = (i as f64 + 0.5) / n;
                let distance = (char_pos - center).abs();
                let intensity = if distance <= half_width {
                    let t = distance / half_width;
                    1.0 - (t * t)
                } else {
                    0.0
                };
                let style = intensity_to_style(intensity);
                Span::styled(c.to_string(), style)
            })
            .collect()
    }
}

fn intensity_to_style(intensity: f64) -> Style {
    let intensity = intensity.clamp(0.0, 1.0);
    let r = lerp(dark::MUTED_FG.0 as f64, dark::PRIMARY.0 as f64, intensity) as u8;
    let g = lerp(dark::MUTED_FG.1 as f64, dark::PRIMARY.1 as f64, intensity) as u8;
    let b = lerp(dark::MUTED_FG.2 as f64, dark::PRIMARY.2 as f64, intensity) as u8;
    let color = Color::Rgb(r, g, b);
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shimmer_styled_spans_length() {
        let s = Shimmer::default();
        let spans = s.styled_spans("locus.");
        assert_eq!(spans.len(), 6);
    }
}