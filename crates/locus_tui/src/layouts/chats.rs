//! Chat area layout: scrollable body region for message list.
//!
//! Uses [crate::utils] for padding and scroll buffer. Does not depend on locus_ui.

use ratatui::layout::Rect;

use crate::utils::{horizontal_padding, scroll_with_buffer, MESSAGE_SPACING_LINES, LEFT_PADDING};

/// Layout for the chat/messages body: outer area and padded inner rect.
#[derive(Debug, Clone)]
pub struct ChatsLayout {
    /// Full body area (e.g. from [super::split::MainSplits::body]).
    pub area: Rect,
    /// Inner rect with horizontal padding for message content.
    pub inner: Rect,
}

impl ChatsLayout {
    /// Build from the body [Rect]. Uses [crate::utils::horizontal_padding].
    pub fn new(area: Rect) -> Self {
        let inner = horizontal_padding(area);
        Self { area, inner }
    }
}

/// Compute scroll offset (clamped so content does not scroll past the viewport).
/// Uses [crate::utils::scroll_with_buffer].
pub fn chat_scroll_offset(
    offset: usize,
    content_height: usize,
    viewport_height: usize,
) -> usize {
    scroll_with_buffer(offset, content_height, viewport_height)
}

/// Re-export for convenience: blank lines to insert between messages.
pub const CHAT_MESSAGE_SPACING: usize = MESSAGE_SPACING_LINES;

/// Left indent string for message continuation lines.
pub const CHAT_LEFT_INDENT: &str = LEFT_PADDING;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chats_layout_inner_has_padding() {
        let area = Rect::new(0, 0, 80, 20);
        let layout = ChatsLayout::new(area);
        assert!(layout.inner.width < area.width);
        assert_eq!(layout.inner.height, area.height);
    }

    #[test]
    fn chats_layout_zero_size() {
        let area = Rect::new(0, 0, 0, 0);
        let layout = ChatsLayout::new(area);
        assert_eq!(layout.inner.width, 0);
        assert_eq!(layout.inner.height, 0);
    }

    #[test]
    fn chat_scroll_offset_no_overflow() {
        // Content fits in viewport — offset should be 0
        assert_eq!(chat_scroll_offset(5, 10, 20), 0);
    }

    #[test]
    fn chat_scroll_offset_clamped() {
        // Scroll beyond content — clamp to max
        let offset = chat_scroll_offset(100, 50, 20);
        assert!(offset <= 50);
    }
}
