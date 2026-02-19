//! Header layout: left = locus.codes, right = git branch • current dir • time.
//!
//! ```text
//! locus.codes                    master • ~/app • 14:32
//! ```

use chrono::{DateTime, Local};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};
use std::path::{Path, PathBuf};

use crate::theme::Theme;

/// Header: left = locus.codes, right = branch • dir • time.
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

    /// Render: outer box (bg only, no padding), inner box (padding, no bg) with content.
    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        // Outer box: background only, no padding (fills full header area)
        let outer = Block::default().style(Style::default().bg(theme.bg));
        f.render_widget(outer, area);

        // Inner box: horizontal padding only for single-line header
        // Vertical padding only if height > 1
        const H_PADDING: u16 = 2;
        const V_PADDING: u16 = 0; // No vertical padding for 1-line header
        let inner = Rect {
            x: area.x.saturating_add(H_PADDING),
            y: area.y.saturating_add(V_PADDING),
            width: area.width.saturating_sub(H_PADDING.saturating_mul(2)),
            height: area.height.saturating_sub(V_PADDING.saturating_mul(2)),
        };

        // Left: locus.codes
        let brand = Span::styled(
            "locus.codes",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        );

        // Right: branch • directory • time
        let mut right_parts: Vec<String> = Vec::new();
        if let Some(ref branch) = self.branch {
            right_parts.push(branch.clone());
        }
        if !self.directory.is_empty() {
            right_parts.push(self.directory.clone());
        }
        right_parts.push(self.time.format("%H:%M").to_string());
        let right_text = right_parts.join(" • ");

        let brand_len = 11;
        let right_len = right_text.chars().count();
        let total_len = inner.width as usize;
        let padding_len = total_len.saturating_sub(brand_len + right_len + 2);

        let line = Line::from(vec![
            brand,
            Span::raw(" ".repeat(padding_len)),
            Span::styled(&right_text, Style::default().fg(theme.faint)),
        ]);

        let paragraph = Paragraph::new(line);
        f.render_widget(paragraph, inner);
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
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

fn format_directory(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            if stripped.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", stripped.display());
        }
    }
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}
