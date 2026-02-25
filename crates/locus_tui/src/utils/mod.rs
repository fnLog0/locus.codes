//! Shared utilities for the locus TUI.
//!
//! - **[constants]** — Spacing, padding, and sizing constants (aligned with locus_ui).
//! - **[layout]** — Rect padding, dynamic height, spacing markers, scroll buffer.
//! - **[format]** — Duration and string truncation for status and messages.

mod constants;
mod format;
mod layout;

pub use constants::*;
pub use format::{
    collapse_repeated_chars, format_duration, truncate_ellipsis, truncate_with_suffix, wrap_lines,
};
pub use layout::{
    collapse_empty_lines,
    dynamic_height,
    horizontal_padding,
    horizontal_padding_with,
    is_spacing_marker,
    padding,
    process_spacing_markers,
    right_aligned_row,
    scroll_with_buffer,
    vertical_padding,
};
