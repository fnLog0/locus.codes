//! Word-level visual diff for AI edits (Medium-style).
//!
//! - Tokenize on whitespace + punctuation boundaries for word-level granularity.
//! - Classify changes: Added (green), Removed (red), Substitute (yellow / removed+added).
//! - Output ratatui spans for inline rendering. No external diff crate; LCS-based token diff.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Maximum total character size to diff; larger texts use line-level fallback to avoid UI freeze.
const MAX_DIFF_CHARS: usize = 8_000;

/// Tokenize text into words and punctuation for word-level diff.
/// Splits on whitespace; keeps punctuation as separate tokens when adjacent to words.
fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if c.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            // Emit space as a token so we can show context
            tokens.push(c.to_string());
        } else if c.is_alphabetic() || c.is_numeric() || c == '_' {
            current.push(c);
        } else {
            // Punctuation/symbol
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            tokens.push(c.to_string());
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// One segment in the diff: either unchanged, added, or removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Unchanged,
    Added,
    Removed,
}

/// Single diff segment (token or group) with its change type.
#[derive(Debug, Clone)]
pub struct DiffSegment {
    pub change: ChangeType,
    pub text: String,
}

/// Max number of added+removed segments to consider a change "simple" (show full inline diff).
pub const SIMPLE_CHANGE_THRESHOLD: usize = 40;

/// Count segments that are added or removed (for "simple change" check).
pub fn count_changed_segments(segments: &[DiffSegment]) -> usize {
    segments
        .iter()
        .filter(|s| s.change != ChangeType::Unchanged)
        .count()
}

/// True when the diff has few added/removed parts — use this to decide whether to show inline diff.
pub fn is_simple_change(old_text: &str, new_text: &str) -> bool {
    let segments = word_diff(old_text, new_text);
    count_changed_segments(&segments) <= SIMPLE_CHANGE_THRESHOLD
}

/// Compute word-level diff between old and new text.
/// Returns a list of segments in display order (removed lines first, then added/unchanged for unified view).
/// For a simple inline view we output: removed tokens (red), then added tokens (green), with unchanged (normal) in order.
/// Actually for "inline" we want one flow: we walk both sequences and emit Remove then Add for changes, Unchanged for same.
pub fn word_diff(old_text: &str, new_text: &str) -> Vec<DiffSegment> {
    if old_text.len() + new_text.len() > MAX_DIFF_CHARS {
        return line_diff_fallback(old_text, new_text);
    }
    let old_tokens = tokenize(old_text);
    let new_tokens = tokenize(new_text);
    lcs_token_diff(&old_tokens, &new_tokens)
}

/// Line-level fallback for large texts: split into lines, diff lines, each line is one segment.
fn line_diff_fallback(old_text: &str, new_text: &str) -> Vec<DiffSegment> {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();
    let old_vec: Vec<String> = old_lines.iter().map(|s| (*s).to_string()).collect();
    let new_vec: Vec<String> = new_lines.iter().map(|s| (*s).to_string()).collect();
    lcs_token_diff(&old_vec, &new_vec)
}

/// One line in a unified diff with file line numbers.
#[derive(Debug, Clone)]
pub struct LineDiffRow {
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
    pub change: ChangeType,
    pub text: String,
}

/// Line-level diff with line numbers for a dedicated diff block (unified style).
pub fn line_diff_with_numbers(old_text: &str, new_text: &str) -> Vec<LineDiffRow> {
    let old_lines: Vec<String> = old_text.lines().map(|s| s.to_string()).collect();
    let new_lines: Vec<String> = new_text.lines().map(|s| s.to_string()).collect();
    let n = old_lines.len();
    let m = new_lines.len();

    if n == 0 && m == 0 {
        return vec![];
    }
    if n == 0 {
        return new_lines
            .into_iter()
            .enumerate()
            .map(|(j, text)| LineDiffRow {
                old_line_no: None,
                new_line_no: Some((j + 1) as u32),
                change: ChangeType::Added,
                text,
            })
            .collect();
    }
    if m == 0 {
        return old_lines
            .into_iter()
            .enumerate()
            .map(|(i, text)| LineDiffRow {
                old_line_no: Some((i + 1) as u32),
                new_line_no: None,
                change: ChangeType::Removed,
                text,
            })
            .collect();
    }

    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if old_lines[i - 1] == new_lines[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let mut out: Vec<LineDiffRow> = Vec::new();
    let mut i = n;
    let mut j = m;
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
            out.push(LineDiffRow {
                old_line_no: Some(i as u32),
                new_line_no: Some(j as u32),
                change: ChangeType::Unchanged,
                text: old_lines[i - 1].clone(),
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            out.push(LineDiffRow {
                old_line_no: None,
                new_line_no: Some(j as u32),
                change: ChangeType::Added,
                text: new_lines[j - 1].clone(),
            });
            j -= 1;
        } else {
            out.push(LineDiffRow {
                old_line_no: Some(i as u32),
                new_line_no: None,
                change: ChangeType::Removed,
                text: old_lines[i - 1].clone(),
            });
            i -= 1;
        }
    }
    out.reverse();
    out
}

/// Render line diff with numbers as ratatui lines: "  old  new | -/+/  content".
const LINE_NO_WIDTH: usize = 4;
const MAX_DIFF_BLOCK_LINES: usize = 50;

pub fn render_line_diff_block(
    rows: &[LineDiffRow],
    style_unchanged: Style,
    style_added: Style,
    style_removed: Style,
    content_width: usize,
) -> Vec<Line<'static>> {
    let mut out = Vec::with_capacity(rows.len().min(MAX_DIFF_BLOCK_LINES + 1));
    for (idx, row) in rows.iter().enumerate() {
        if idx >= MAX_DIFF_BLOCK_LINES {
            out.push(Line::from(Span::styled(
                format!(
                    "{:>3$} {:>3$} │ … ({} more lines)",
                    "",
                    "",
                    rows.len() - MAX_DIFF_BLOCK_LINES,
                    LINE_NO_WIDTH
                ),
                style_unchanged,
            )));
            break;
        }
        let old_s = row
            .old_line_no
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        let new_s = row
            .new_line_no
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        let prefix = format!("{:>2$} {:>2$} │ ", old_s, new_s, LINE_NO_WIDTH);
        let marker = match row.change {
            ChangeType::Unchanged => "  ",
            ChangeType::Added => "+ ",
            ChangeType::Removed => "- ",
        };
        let style = match row.change {
            ChangeType::Unchanged => style_unchanged,
            ChangeType::Added => style_added,
            ChangeType::Removed => style_removed,
        };
        let content = if row.text.len() > content_width {
            format!(
                "{}…",
                row.text
                    .chars()
                    .take(content_width.saturating_sub(1))
                    .collect::<String>()
            )
        } else {
            row.text.clone()
        };
        let spans = vec![
            Span::styled(prefix, style_unchanged),
            Span::styled(marker, style),
            Span::styled(content, style),
        ];
        out.push(Line::from(spans));
    }
    out
}

/// LCS-based diff: walk the LCS table to produce Remove / Add / Unchanged segments.
fn lcs_token_diff(old: &[String], new: &[String]) -> Vec<DiffSegment> {
    let n = old.len();
    let m = new.len();
    if n == 0 && m == 0 {
        return vec![];
    }
    if n == 0 {
        return new
            .iter()
            .map(|s| DiffSegment {
                change: ChangeType::Added,
                text: s.clone(),
            })
            .collect();
    }
    if m == 0 {
        return old
            .iter()
            .map(|s| DiffSegment {
                change: ChangeType::Removed,
                text: s.clone(),
            })
            .collect();
    }

    // dp[i][j] = LCS length for old[0..i], new[0..j]
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if old[i - 1] == new[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to build segment list (reverse order then reverse)
    let mut out: Vec<DiffSegment> = Vec::new();
    let mut i = n;
    let mut j = m;
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
            out.push(DiffSegment {
                change: ChangeType::Unchanged,
                text: old[i - 1].clone(),
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            out.push(DiffSegment {
                change: ChangeType::Added,
                text: new[j - 1].clone(),
            });
            j -= 1;
        } else {
            out.push(DiffSegment {
                change: ChangeType::Removed,
                text: old[i - 1].clone(),
            });
            i -= 1;
        }
    }
    out.reverse();
    out
}

/// Convert diff segments to ratatui lines with color encoding:
/// green = added, red = removed, default = unchanged.
/// Wraps by width; each line is a [Line]. Max lines capped for performance.
pub fn diff_to_lines(
    segments: &[DiffSegment],
    style_unchanged: Style,
    style_added: Style,
    style_removed: Style,
    width: usize,
    max_lines: usize,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();
    let mut current_width = 0usize;

    for seg in segments {
        let style = match seg.change {
            ChangeType::Unchanged => style_unchanged,
            ChangeType::Added => style_added,
            ChangeType::Removed => style_removed,
        };
        let w = unicode_width::UnicodeWidthStr::width(seg.text.as_str());
        if current_width + w > width && !current_line.is_empty() {
            lines.push(Line::from(std::mem::take(&mut current_line)));
            current_width = 0;
            if lines.len() >= max_lines {
                current_line.push(Span::styled("…", style_unchanged));
                lines.push(Line::from(current_line));
                return lines;
            }
        }
        current_line.push(Span::styled(seg.text.clone(), style));
        current_width += w;
    }
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }
    lines
}

/// High-level: compute word-level diff and return ratatui lines for the TUI.
pub fn render_diff_lines(
    old_content: &str,
    new_content: &str,
    style_unchanged: Style,
    style_added: Style,
    style_removed: Style,
    width: usize,
    max_lines: usize,
) -> Vec<Line<'static>> {
    let segments = word_diff(old_content, new_content);
    diff_to_lines(
        &segments,
        style_unchanged,
        style_added,
        style_removed,
        width,
        max_lines,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple() {
        let t = tokenize("hello, world!");
        assert!(t.len() >= 3);
        assert!(t.contains(&"hello".to_string()));
        assert!(t.contains(&",".to_string()));
    }

    #[test]
    fn word_diff_unchanged() {
        let segs = word_diff("a b c", "a b c");
        assert_eq!(segs.len(), 5); // a space b space c
        assert!(segs.iter().all(|s| s.change == ChangeType::Unchanged));
    }

    #[test]
    fn word_diff_added() {
        let segs = word_diff("a b", "a b c");
        assert!(
            segs.iter()
                .any(|s| s.change == ChangeType::Added && s.text == "c")
        );
    }

    #[test]
    fn word_diff_removed() {
        let segs = word_diff("a b c", "a b");
        assert!(
            segs.iter()
                .any(|s| s.change == ChangeType::Removed && s.text == "c")
        );
    }

    #[test]
    fn simple_change_small_edit() {
        assert!(is_simple_change("hello", "hello world"));
        assert!(is_simple_change(
            "fn foo() {}",
            "fn foo() { /* comment */ }"
        ));
    }

    #[test]
    fn not_simple_change_many_tokens() {
        let old: String = (0..50).map(|i| format!("word{} ", i)).collect();
        let new: String = (0..50).map(|i| format!("x{} ", i)).collect();
        assert!(!is_simple_change(&old, &new));
    }
}
