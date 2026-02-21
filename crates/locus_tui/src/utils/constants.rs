//! TUI spacing and sizing constants.
//!
//! Kept in sync with locus_ui for consistent layout. Use these when building
//! layout or rendering in locus_tui so padding and spacing stay uniform.

/// Lines to keep visible at scroll edges (content does not touch viewport edge).
pub const SCROLL_BUFFER_LINES: usize = 2;

/// Maximum height for dropdown menus (in lines).
pub const DROPDOWN_MAX_HEIGHT: usize = 8;

/// Maximum height of popups as fraction of terminal height (0.0–1.0).
pub const POPUP_MAX_HEIGHT_PERCENT: f32 = 0.6;

/// Horizontal padding in characters (each side).
pub const HORIZONTAL_PADDING: u16 = 2;

/// Left indent for side panels and indented content (two spaces).
pub const LEFT_PADDING: &str = "  ";

/// Minimum height for dynamic components (e.g. popups, dropdowns).
pub const MIN_COMPONENT_HEIGHT: u16 = 3;

/// Blank lines between message blocks.
pub const MESSAGE_SPACING_LINES: usize = 1;

/// Maximum consecutive empty lines allowed (avoids huge vertical gaps).
pub const MAX_CONSECUTIVE_EMPTY_LINES: usize = 2;

/// Marker string for injecting spacing in message content; replaced with empty lines when rendering.
pub const SPACING_MARKER: &str = "SPACING_MARKER";
