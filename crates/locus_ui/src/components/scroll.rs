//! Generic scrollable panel state.
//!
//! Tracks scroll offset and content height for any scrollable region.

/// Scroll position state for a panel.
#[derive(Debug, Clone)]
pub struct ScrollPanel {
    /// Current vertical scroll offset (lines hidden at top).
    pub offset: usize,
    /// Total height of content in lines.
    pub content_height: usize,
    /// Viewport height in lines.
    pub viewport_height: usize,
    /// Whether to auto-scroll to bottom on new content.
    pub auto_scroll: bool,
}

impl ScrollPanel {
    /// Create a new scroll panel with default settings.
    pub fn new() -> Self {
        Self {
            offset: 0,
            content_height: 0,
            viewport_height: 0,
            auto_scroll: true,
        }
    }

    /// Set the content height and optionally auto-scroll.
    pub fn set_content_height(&mut self, height: usize) {
        let was_at_bottom = self.content_height > 0 && self.is_at_bottom();
        self.content_height = height;

        if self.auto_scroll && was_at_bottom {
            self.scroll_to_bottom();
        }
    }

    /// Set the viewport height.
    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
        // Clamp offset if viewport grew
        self.clamp_offset();
    }

    /// Scroll up by a number of lines.
    pub fn scroll_up(&mut self, lines: usize) {
        self.offset = self.offset.saturating_sub(lines);
    }

    /// Scroll down by a number of lines.
    pub fn scroll_down(&mut self, lines: usize) {
        self.offset = self.offset.saturating_add(lines);
        self.clamp_offset();
    }

    /// Scroll up by a page (viewport height).
    pub fn page_up(&mut self) {
        self.scroll_up(self.viewport_height.saturating_sub(1));
    }

    /// Scroll down by a page (viewport height).
    pub fn page_down(&mut self) {
        self.scroll_down(self.viewport_height.saturating_sub(1));
    }

    /// Scroll to the top of the content.
    pub fn scroll_to_top(&mut self) {
        self.offset = 0;
    }

    /// Scroll to the bottom of the content.
    pub fn scroll_to_bottom(&mut self) {
        let max_offset = self.max_offset();
        self.offset = max_offset;
    }

    /// Check if currently at the top of the content.
    pub fn is_at_top(&self) -> bool {
        self.offset == 0
    }

    /// Check if currently at the bottom of the content.
    pub fn is_at_bottom(&self) -> bool {
        self.offset >= self.max_offset()
    }

    /// Check if content exceeds viewport (scrolling needed).
    pub fn needs_scroll(&self) -> bool {
        self.content_height > self.viewport_height
    }

    /// Get the maximum allowed offset.
    fn max_offset(&self) -> usize {
        self.content_height.saturating_sub(self.viewport_height)
    }

    /// Clamp offset to valid range.
    fn clamp_offset(&mut self) {
        let max = self.max_offset();
        if self.offset > max {
            self.offset = max;
        }
    }

    /// Enable or disable auto-scroll.
    pub fn set_auto_scroll(&mut self, enabled: bool) {
        self.auto_scroll = enabled;
    }

    /// Get scroll indicator text based on position.
    pub fn indicator(&self) -> Option<ScrollIndicator> {
        if !self.needs_scroll() {
            return None;
        }

        if self.is_at_top() {
            Some(ScrollIndicator::CanScrollDown)
        } else if self.is_at_bottom() {
            Some(ScrollIndicator::CanScrollUp)
        } else {
            Some(ScrollIndicator::CanScrollBoth)
        }
    }
}

impl Default for ScrollPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Scroll indicator type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollIndicator {
    /// At top, can scroll down.
    CanScrollDown,
    /// At bottom, can scroll up.
    CanScrollUp,
    /// In middle, can scroll both ways.
    CanScrollBoth,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_panel_basic() {
        let mut panel = ScrollPanel::new();
        panel.set_viewport_height(10);
        panel.set_content_height(50);

        assert!(panel.needs_scroll());
        assert!(panel.is_at_top());
        assert!(!panel.is_at_bottom());
    }

    #[test]
    fn scroll_down_clamps_at_bottom() {
        let mut panel = ScrollPanel::new();
        panel.set_viewport_height(10);
        panel.set_content_height(20);

        panel.scroll_down(100);
        assert_eq!(panel.offset, 10); // max offset = 20 - 10
    }

    #[test]
    fn auto_scroll_on_new_content() {
        let mut panel = ScrollPanel::new();
        panel.set_viewport_height(10);
        panel.set_content_height(20);
        panel.scroll_to_bottom();

        assert!(panel.is_at_bottom());

        // Add more content
        panel.set_content_height(30);
        assert!(panel.is_at_bottom()); // Still at bottom due to auto_scroll
    }

    #[test]
    fn no_scroll_needed() {
        let mut panel = ScrollPanel::new();
        panel.set_viewport_height(20);
        panel.set_content_height(10);

        assert!(!panel.needs_scroll());
        assert!(panel.is_at_top());
        assert!(panel.is_at_bottom());
    }
}
