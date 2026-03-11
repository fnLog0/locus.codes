//! Memory event UI: display memory recall and storage events from LocusGraph.
//!
//! Types are TUI-only (no dependency on locus_runtime). The runtime maps
//! memory events to [MemoryMessage] for display. Colors from [crate::theme] only.

use ratatui::text::{Line, Span};

use crate::layouts::{text_muted_style, text_style};
use crate::theme::LocusPalette;

const MEMORY_LEFT_BORDER: &str = "· ";

/// Kind of memory event for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    /// Recalled memories from LocusGraph.
    Recall,
    /// Stored a new memory to LocusGraph.
    Store,
}

impl MemoryKind {
    /// Display label in the UI.
    pub fn label(self) -> &'static str {
        match self {
            MemoryKind::Recall => "Memory recall",
            MemoryKind::Store => "Memory store",
        }
    }

    /// Icon for the event.
    pub fn icon(self) -> &'static str {
        match self {
            MemoryKind::Recall => "◎",
            MemoryKind::Store => "◉",
        }
    }
}

/// One memory event to show in the chat/log.
#[derive(Debug, Clone)]
pub struct MemoryMessage {
    pub kind: MemoryKind,
    /// For recall: the search query. For store: the context_id.
    pub context: String,
    /// For recall: number of items found. For store: the event_kind.
    pub detail: String,
    /// Optional summary preview (first N chars of stored data or top recall).
    pub summary: Option<String>,
}

impl MemoryMessage {
    /// Create a memory recall message.
    pub fn recall(query: impl Into<String>, items_found: u64) -> Self {
        Self {
            kind: MemoryKind::Recall,
            context: query.into(),
            detail: format!("{} memories", items_found),
            summary: None,
        }
    }

    /// Create a memory store message.
    pub fn store(
        context_id: impl Into<String>,
        event_kind: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        let summary_str = summary.into();
        Self {
            kind: MemoryKind::Store,
            context: context_id.into(),
            detail: event_kind.into(),
            summary: if summary_str.is_empty() {
                None
            } else {
                Some(summary_str)
            },
        }
    }
}

/// Build a single [Line] for a memory event.
pub fn memory_line(msg: &MemoryMessage, palette: &LocusPalette) -> Line<'static> {
    let mut spans = vec![Span::styled(
        MEMORY_LEFT_BORDER.to_string(),
        text_muted_style(palette.border_variant),
    )];

    // Icon
    spans.push(Span::styled(
        format!("{} ", msg.kind.icon()),
        text_style(palette.info),
    ));

    // Label
    spans.push(Span::styled(
        msg.kind.label().to_string(),
        text_muted_style(palette.text_muted),
    ));

    // Context (query for recall, context_id for store)
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        truncate_context(&msg.context, 40),
        text_muted_style(palette.text_muted),
    ));

    // Detail
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        msg.detail.clone(),
        text_muted_style(palette.text_muted),
    ));

    // Optional summary preview
    if let Some(ref summary) = msg.summary {
        let preview = truncate_summary(summary, 50);
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("\"{}\"", preview),
            text_muted_style(palette.text_muted),
        ));
    }

    Line::from(spans)
}

/// Truncate context_id or query for display.
fn truncate_context(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_len {
        s.to_string()
    } else {
        let truncated: String = chars.iter().take(max_len.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

/// Truncate summary for display.
fn truncate_summary(s: &str, max_len: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    let was_truncated = first_line.chars().count() > max_len;
    let truncated: String = first_line.chars().take(max_len).collect();
    if was_truncated {
        format!("{}…", truncated)
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_recall_message() {
        let msg = MemoryMessage::recall("fix JWT authentication", 3);
        assert_eq!(msg.kind, MemoryKind::Recall);
        assert_eq!(msg.context, "fix JWT authentication");
        assert_eq!(msg.detail, "3 memories");
    }

    #[test]
    fn memory_store_message() {
        let msg = MemoryMessage::store(
            "intent:session_turn001",
            "observation",
            "user wants to fix JWT",
        );
        assert_eq!(msg.kind, MemoryKind::Store);
        assert_eq!(msg.context, "intent:session_turn001");
        assert_eq!(msg.detail, "observation");
        assert_eq!(msg.summary, Some("user wants to fix JWT".to_string()));
    }

    #[test]
    fn memory_store_empty_summary() {
        let msg = MemoryMessage::store("turn:abc123", "fact", "");
        assert_eq!(msg.kind, MemoryKind::Store);
        assert!(msg.summary.is_none());
    }

    #[test]
    fn memory_line_builds() {
        let msg = MemoryMessage::recall("search query", 5);
        let palette = LocusPalette::locus_dark();
        let line = memory_line(&msg, &palette);
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn truncate_context_short() {
        assert_eq!(truncate_context("short", 40), "short");
    }

    #[test]
    fn truncate_context_long() {
        let long = "this_is_a_very_long_context_id_that_needs_truncation";
        let truncated = truncate_context(long, 20);
        assert!(truncated.chars().count() <= 20);
        assert!(truncated.ends_with('…'));
    }

    #[test]
    fn truncate_summary_adds_ellipsis_when_needed() {
        let summary = truncate_summary("abcdefghijklmnopqrstuvwxyz", 10);
        assert_eq!(summary, "abcdefghij…");
    }
}
