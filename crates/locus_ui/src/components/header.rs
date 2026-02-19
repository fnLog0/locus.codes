//! Header component - top bar with brand and context info.
//!
//! Layout:
//! ```text
//! locus.codes                              master • ~/app • 14:32
//! ```

use chrono::{DateTime, Local};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::path::{Path, PathBuf};

use crate::theme::Theme;

/// Header showing brand and context information.
#[derive(Debug, Clone)]
pub struct Header {
    /// Git branch (if in a git repo).
    pub branch: Option<String>,
    /// Current directory (displayed as ~/relative or basename).
    pub directory: String,
    /// Current time (updated periodically).
    pub time: DateTime<Local>,
}

impl Header {
    /// Create a new header with current time.
    pub fn new() -> Self {
        Self {
            branch: None,
            directory: String::new(),
            time: Local::now(),
        }
    }

    /// Update the current time.
    pub fn update_time(&mut self) {
        self.time = Local::now();
    }

    /// Update the git branch.
    pub fn update_branch(&mut self, branch: Option<String>) {
        self.branch = branch;
    }

    /// Update the current directory.
    /// Converts to ~/relative format when in home directory.
    pub fn update_directory(&mut self, dir: PathBuf) {
        self.directory = format_directory(&dir);
    }

    /// Render the header into the frame.
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        // Left side: brand "locus.codes"
        let brand = Span::styled(
            "locus.codes",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        );

        // Right side: branch • directory • time
        let mut right_parts: Vec<String> = Vec::new();

        if let Some(ref branch) = self.branch {
            right_parts.push(branch.clone());
        }

        if !self.directory.is_empty() {
            right_parts.push(self.directory.clone());
        }

        right_parts.push(self.time.format("%H:%M").to_string());

        let right_text = right_parts.join(" • ");

        // Build the line with proper spacing
        let brand_len = 11; // "locus.codes"
        let right_len = right_text.chars().count();

        // Calculate available space for middle padding
        let total_len = area.width as usize;
        let padding_len = total_len.saturating_sub(brand_len + right_len + 2);

        let line = Line::from(vec![
            brand,
            Span::raw(" ".repeat(padding_len)),
            Span::styled(&right_text, Style::default().fg(theme.muted_fg)),
        ]);

        let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg));
        f.render_widget(paragraph, area);
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a directory path for display.
/// Shows ~/relative when in home, otherwise just the basename.
fn format_directory(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            if stripped.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", stripped.display());
        }
    }

    // Fallback to basename
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_new() {
        let header = Header::new();
        assert!(header.branch.is_none());
        assert!(header.directory.is_empty());
    }

    #[test]
    fn header_update_branch() {
        let mut header = Header::new();
        header.update_branch(Some("master".to_string()));
        assert_eq!(header.branch, Some("master".to_string()));
    }
}
