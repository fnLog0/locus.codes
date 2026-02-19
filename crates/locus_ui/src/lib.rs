//! locus-ui — terminal UI for locus.codes (Ratatui + Crossterm).
//!
//! Provides loading screen, components, and animation utilities.

pub mod animation;
pub mod app_state;
pub mod components;
pub mod main_view;
pub mod theme;

pub use animation::Shimmer;
pub use app_state::AppState;
pub use components::{
    Chat, ContentBlock, Grid, Header, Input, Loader, Message, Popup, Role, ScrollIndicator,
    ScrollPanel, ShellPopup, ShortcutsBar, Spinner, ToolDisplay, ToolStatus,
};
pub use main_view::view;
pub use theme::{Theme, ThemeMode};
