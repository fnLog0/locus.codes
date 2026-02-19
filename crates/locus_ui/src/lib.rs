//! locus-ui — terminal UI for locus.codes (Ratatui + Crossterm).
//!
//! Provides loading screen, components, and animation utilities.

pub mod animation;
pub mod components;
pub mod theme;

pub use animation::Shimmer;
pub use components::{
    Chat, ContentBlock, Grid, Header, Input, Loader, Message, Role, ScrollIndicator, ScrollPanel,
    ShortcutsBar, Spinner, ToolDisplay, ToolStatus,
};
pub use theme::{Theme, ThemeMode};
