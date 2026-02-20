//! Reusable UI components (shadcn-like: variants, composition, theme tokens).

mod chat;
mod constants;
mod grid;
mod header;
mod input;
mod layout;
mod loader;
mod messages;
mod pixel_font;
mod popup;
mod scroll;
mod shortcuts;
mod spinner;

pub use chat::Chat;
pub use constants::{
    DROPDOWN_MAX_HEIGHT, HORIZONTAL_PADDING, LEFT_PADDING, MAX_CONSECUTIVE_EMPTY_LINES,
    MESSAGE_SPACING_LINES, MIN_COMPONENT_HEIGHT, POPUP_MAX_HEIGHT_PERCENT, SCROLL_BUFFER_LINES,
    SPACING_MARKER,
};
pub use grid::Grid;
pub use header::Header;
pub use input::Input;
pub use layout::{
    collapse_empty_lines, dynamic_height, horizontal_padding, horizontal_padding_with, padding,
    process_spacing_markers, right_aligned_row, scroll_with_buffer, vertical_padding,
};
pub use loader::Loader;
pub use messages::{ContentBlock, Message, Role, ToolDisplay, ToolStatus};
pub use popup::{Popup, ShellPopup};
pub use scroll::{ScrollIndicator, ScrollPanel};
pub use shortcuts::ShortcutsBar;
pub use spinner::Spinner;
