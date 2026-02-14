use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use regex::Regex;

use crate::detect_term::AdaptiveColors;

/// Simplified markdown renderer for Phase 0
/// Renders basic markdown elements like headers, bold, code blocks, etc.

/// Render markdown text to ratatui lines
pub fn render_markdown_to_lines_with_width(
    markdown: &str,
    width: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Simple markdown parsing
    for line in markdown.lines() {
        if line.trim().is_empty() {
            lines.push(Line::from(""));
            continue;
        }

        // Headers (# ## ###)
        if line.starts_with("# ") {
            let text = line[2..].to_string();
            lines.push(Line::from(vec![
            Span::styled(text, Style::default().fg(AdaptiveColors::text()).add_modifier(Modifier::BOLD)),
            ]));
        } else if line.starts_with("## ") {
            let text = line[3..].to_string();
            lines.push(Line::from(vec![
                Span::styled(text, Style::default().fg(AdaptiveColors::text())),
            ]));
        } else if line.starts_with("### ") {
            let text = line[4..].to_string();
            lines.push(Line::from(vec![
                Span::styled(text, Style::default().fg(AdaptiveColors::text())),
            ]));
        }
        // Bold text **text**
        else if let Some(captures) = Regex::new(r"\*\*(.+?)\*\*").unwrap().captures(line) {
            let before = line[..captures.get(0).unwrap().start()].to_string();
            let bold = captures[1].to_string();
            let after = line[captures.get(0).unwrap().end()..].to_string();
            lines.push(Line::from(vec![
                Span::styled(before, Style::default().fg(AdaptiveColors::text())),
                Span::styled(bold, Style::default().fg(AdaptiveColors::text()).add_modifier(Modifier::BOLD)),
                Span::styled(after, Style::default().fg(AdaptiveColors::text())),
            ]));
        }
        // Italic text *text*
        else if let Some(captures) = Regex::new(r"\*(.+?)\*").unwrap().captures(line) {
            let before = line[..captures.get(0).unwrap().start()].to_string();
            let italic = captures[1].to_string();
            let after = line[captures.get(0).unwrap().end()..].to_string();
            lines.push(Line::from(vec![
                Span::styled(before, Style::default().fg(AdaptiveColors::text())),
                Span::styled(italic, Style::default().fg(AdaptiveColors::text()).add_modifier(Modifier::ITALIC)),
                Span::styled(after, Style::default().fg(AdaptiveColors::text())),
            ]));
        }
        // Code block ```lang ... ```
        else if line.starts_with("```") {
            // Start/end of code block
            lines.push(Line::from(vec![
                Span::styled(line.to_string(), Style::default().fg(AdaptiveColors::green())),
            ]));
        }
        // Inline code `code`
        else if line.contains('`') {
            let parts: Vec<&str> = line.split('`').collect();
            let mut spans = Vec::new();
            for (i, part) in parts.iter().enumerate() {
                if i % 2 == 1 {
                    // Inside backticks - code
                    spans.push(Span::styled(part.to_string(), Style::default().fg(AdaptiveColors::green())));
                } else {
                    // Outside backticks - normal text
                    spans.push(Span::styled(part.to_string(), Style::default().fg(AdaptiveColors::text())));
                }
            }
            lines.push(Line::from(spans));
        }
        // Horizontal rule ---
        else if line.trim() == "---" || line.trim() == "***" {
            lines.push(Line::from(vec![
                Span::styled("─".repeat(width), Style::default().fg(AdaptiveColors::dark_gray())),
            ]));
        }
        // Bullet points - * or -
        else if line.starts_with("* ") || line.starts_with("- ") {
            let text = line[2..].to_string();
            lines.push(Line::from(vec![
                Span::styled("• ".to_string(), Style::default().fg(AdaptiveColors::orange())),
                Span::styled(text, Style::default().fg(AdaptiveColors::text())),
            ]));
        }
        // Numbered lists 1. 2. etc.
        else if Regex::new(r"^\d+\.\s").unwrap().is_match(line) {
            if let Some(captures) = Regex::new(r"^(\d+\.)\s(.+)$").unwrap().captures(line) {
                let num = captures[1].to_string();
                let text = captures[2].to_string();
                lines.push(Line::from(vec![
                    Span::styled(format!("{} ", num), Style::default().fg(AdaptiveColors::orange())),
                    Span::styled(text, Style::default().fg(AdaptiveColors::text())),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(line.to_string(), Style::default().fg(AdaptiveColors::text())),
                ]));
            }
        }
        // Blockquotes > text
        else if line.starts_with("> ") {
            let text = line[2..].to_string();
            lines.push(Line::from(vec![
                Span::styled("│ ".to_string(), Style::default().fg(AdaptiveColors::dark_magenta())),
                Span::styled(text, Style::default().fg(AdaptiveColors::light_magenta())),
            ]));
        }
        // Regular paragraph
        else {
            lines.push(Line::from(vec![
                Span::styled(line.to_string(), Style::default().fg(AdaptiveColors::text())),
            ]));
        }
    }

    lines
}
