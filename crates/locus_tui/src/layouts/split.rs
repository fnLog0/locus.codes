//! Split the terminal area into header, body, and footer regions.
//!
//! Uses [utils] constants for minimum heights; you can override heights
//! when calling the split functions.

use ratatui::layout::Rect;

use crate::utils::horizontal_padding;

/// Fixed height for the header (top, two lines: title + border).
pub const HEADER_HEIGHT: u16 = 2;

/// Fixed height for the footer: input block (3 lines: border + content + border) + shortcut line.
pub const FOOTER_HEIGHT: u16 = 4;

/// Regions for a main app layout: header, scrollable body, footer.
#[derive(Debug, Clone)]
pub struct MainSplits {
    /// Top strip (e.g. title, tabs).
    pub header: Rect,
    /// Middle area (e.g. chat, content). May have zero height if area too small.
    pub body: Rect,
    /// Bottom strip (e.g. status, input hint).
    pub footer: Rect,
}

/// Split `area` into header (fixed top), body (scrollable middle), footer (fixed bottom).
/// Uses [HEADER_HEIGHT] and [FOOTER_HEIGHT]. Body height = area.height - header - footer.
pub fn main_splits(area: Rect) -> MainSplits {
    let height = area.height;
    let (header_h, footer_h) = (HEADER_HEIGHT, FOOTER_HEIGHT);
    let body_h = height.saturating_sub(header_h + footer_h);

    let header = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: header_h,
    };
    let body = Rect {
        x: area.x,
        y: area.y.saturating_add(header_h),
        width: area.width,
        height: body_h,
    };
    let footer = Rect {
        x: area.x,
        y: area.y.saturating_add(header_h + body_h),
        width: area.width,
        height: footer_h,
    };

    MainSplits {
        header,
        body,
        footer,
    }
}

/// Same as [main_splits] but body is the padded inner area (horizontal padding only).
pub fn main_splits_with_padding(area: Rect) -> MainSplits {
    let raw = main_splits(area);
    MainSplits {
        header: raw.header,
        body: horizontal_padding(raw.body),
        footer: raw.footer,
    }
}

/// Split a vertical strip into top and bottom with a given top height.
pub fn vertical_split(area: Rect, top_height: u16) -> (Rect, Rect) {
    let top_h = top_height.min(area.height);
    let bottom_h = area.height.saturating_sub(top_h);
    let top = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: top_h,
    };
    let bottom = Rect {
        x: area.x,
        y: area.y.saturating_add(top_h),
        width: area.width,
        height: bottom_h,
    };
    (top, bottom)
}

/// Split horizontally into left and right with a given left width.
pub fn horizontal_split(area: Rect, left_width: u16) -> (Rect, Rect) {
    let left_w = left_width.min(area.width);
    let right_w = area.width.saturating_sub(left_w);
    let left = Rect {
        x: area.x,
        y: area.y,
        width: left_w,
        height: area.height,
    };
    let right = Rect {
        x: area.x.saturating_add(left_w),
        y: area.y,
        width: right_w,
        height: area.height,
    };
    (left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_splits_assigns_regions() {
        let area = Rect::new(0, 0, 80, 24);
        let s = main_splits(area);
        assert_eq!(s.header.height, 2);
        assert_eq!(s.footer.height, 4);
        assert_eq!(s.body.height, 18);
        assert_eq!(s.body.y, 2);
        assert_eq!(s.footer.y, 20);
    }

    #[test]
    fn vertical_split_divides_height() {
        let area = Rect::new(0, 0, 80, 10);
        let (top, bottom) = vertical_split(area, 3);
        assert_eq!(top.height, 3);
        assert_eq!(bottom.height, 7);
        assert_eq!(bottom.y, 3);
    }

    #[test]
    fn horizontal_split_divides_width() {
        let area = Rect::new(0, 0, 80, 24);
        let (left, right) = horizontal_split(area, 20);
        assert_eq!(left.width, 20);
        assert_eq!(right.width, 60);
        assert_eq!(right.x, 20);
    }

    #[test]
    fn main_splits_tiny_terminal() {
        let area = Rect::new(0, 0, 80, 3);
        let s = main_splits(area);
        // Body should collapse to 0 when terminal too small
        assert_eq!(s.body.height, 0);
        assert_eq!(s.header.height, HEADER_HEIGHT);
    }

    #[test]
    fn main_splits_exact_minimum() {
        let area = Rect::new(0, 0, 80, HEADER_HEIGHT + FOOTER_HEIGHT);
        let s = main_splits(area);
        assert_eq!(s.body.height, 0);
    }

    #[test]
    fn vertical_split_larger_than_area() {
        let area = Rect::new(0, 0, 80, 5);
        let (top, bottom) = vertical_split(area, 10);
        assert_eq!(top.height, 5);
        assert_eq!(bottom.height, 0);
    }

    #[test]
    fn horizontal_split_zero_width() {
        let area = Rect::new(0, 0, 0, 24);
        let (left, right) = horizontal_split(area, 10);
        assert_eq!(left.width, 0);
        assert_eq!(right.width, 0);
    }
}
