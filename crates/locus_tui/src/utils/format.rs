//! Formatting helpers for TUI text (durations, truncation).
//!
//! Use these when rendering status lines, tool timing, or any fixed-width text.

use std::time::Duration;

/// Format a duration for display (e.g. "123ms", "2s 450ms").
///
/// Uses milliseconds when under 1s, otherwise seconds and milliseconds.
pub fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        let s = ms / 1000;
        let rest_ms = ms % 1000;
        if rest_ms == 0 {
            format!("{}s", s)
        } else {
            format!("{}s {}ms", s, rest_ms)
        }
    }
}

/// Truncate `s` to at most `max_width` characters, appending `suffix` when truncated.
/// Uses character count (not grapheme clusters); suitable for terminal column width in simple cases.
pub fn truncate_with_suffix(s: &str, max_width: usize, suffix: &str) -> String {
    if s.len() <= max_width {
        return s.to_string();
    }
    let suffix_len = suffix.len();
    if max_width <= suffix_len {
        return suffix.chars().take(max_width).collect();
    }
    let take = max_width - suffix_len;
    format!("{}{}", s.chars().take(take).collect::<String>(), suffix)
}

/// Truncate to `max_width` with "…" suffix when needed.
#[inline]
pub fn truncate_ellipsis(s: &str, max_width: usize) -> String {
    truncate_with_suffix(s, max_width, "…")
}

/// Collapse long runs of the same character to avoid walls of `}}}}...` from runaway LLM output.
/// Runs longer than `max_repeat` become e.g. `}… (×47)`.
pub fn collapse_repeated_chars(s: &str, max_repeat: usize) -> String {
    if max_repeat == 0 || s.is_empty() {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        let mut count = 1usize;
        while chars.peek() == Some(&c) {
            chars.next();
            count += 1;
        }
        if count > max_repeat {
            out.push(c);
            out.push_str("… (×");
            out.push_str(&count.to_string());
            out.push(')');
        } else {
            for _ in 0..count {
                out.push(c);
            }
        }
    }
    out
}

/// Word-wrap text to lines of at most `width` characters (by word boundary).
/// Long words are pushed as their own line. Returns empty vec for empty or whitespace-only input.
pub fn wrap_lines(s: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        let need = if line.is_empty() {
            word.len()
        } else {
            line.len() + 1 + word.len()
        };
        if need <= width {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        } else {
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }
            if word.len() <= width {
                line = word.to_string();
            } else {
                out.push(word.to_string());
            }
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_ms() {
        assert_eq!(format_duration(Duration::from_millis(0)), "0ms");
        assert_eq!(format_duration(Duration::from_millis(123)), "123ms");
        assert_eq!(format_duration(Duration::from_millis(999)), "999ms");
    }

    #[test]
    fn format_duration_seconds() {
        assert_eq!(format_duration(Duration::from_millis(1000)), "1s");
        assert_eq!(format_duration(Duration::from_millis(2500)), "2s 500ms");
    }

    #[test]
    fn truncate_ellipsis_short() {
        assert_eq!(truncate_ellipsis("hi", 10), "hi");
        assert_eq!(truncate_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn truncate_ellipsis_long() {
        // Ellipsis "…" is 1 char but 3 bytes; we reserve suffix by byte length so 8 - 3 = 5 chars
        assert_eq!(truncate_ellipsis("hello world", 8), "hello…");
        assert_eq!(truncate_ellipsis("ab", 1), "…");
    }

    #[test]
    fn wrap_lines_by_width() {
        let lines = wrap_lines("one two three four", 8);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "one two");
        assert_eq!(lines[1], "three");
        assert_eq!(lines[2], "four");
    }

    #[test]
    fn wrap_lines_empty() {
        assert!(wrap_lines("", 10).is_empty());
        assert!(wrap_lines("   ", 10).is_empty());
    }

    #[test]
    fn collapse_repeated_short_run_unchanged() {
        assert_eq!(collapse_repeated_chars("}}}", 4), "}}}");
        assert_eq!(collapse_repeated_chars("ab", 4), "ab");
    }

    #[test]
    fn collapse_repeated_long_run() {
        assert_eq!(collapse_repeated_chars("}}}}}", 4), "}… (×5)");
        assert_eq!(collapse_repeated_chars("xaaaaay", 3), "xa… (×5)y");
    }
}
