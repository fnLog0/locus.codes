//! UI constants for consistent spacing and sizing across components.
//!
//! Based on Refactoring UI principles and patterns from stakpak/agent.

/// Lines to keep visible at scroll edges (prevents content from touching viewport edge).
pub const SCROLL_BUFFER_LINES: usize = 2;

/// Maximum height for dropdown menus.
pub const DROPDOWN_MAX_HEIGHT: usize = 8;

/// Maximum height percentage for popup windows (0.0 - 1.0).
pub const POPUP_MAX_HEIGHT_PERCENT: f32 = 0.6;

/// Horizontal padding (characters on each side).
pub const HORIZONTAL_PADDING: u16 = 2;

/// Left padding string for side panels and indented content.
pub const LEFT_PADDING: &str = "  ";

/// Minimum height for dynamic components.
pub const MIN_COMPONENT_HEIGHT: u16 = 3;

/// Standard spacing between message blocks.
pub const MESSAGE_SPACING_LINES: usize = 1;

/// Maximum consecutive empty lines to allow (prevents huge gaps).
pub const MAX_CONSECUTIVE_EMPTY_LINES: usize = 2;

/// Marker string for spacing injection in message rendering.
pub const SPACING_MARKER: &str = "SPACING_MARKER";
