//! Reusable UI components (shadcn-like: variants, composition, theme tokens).

mod chat;
mod grid;
mod header;
mod input;
mod loader;
mod message;
mod pixel_font;
mod scroll;
mod shortcuts;
mod spinner;

pub use chat::Chat;
pub use grid::Grid;
pub use header::Header;
pub use input::Input;
pub use loader::Loader;
pub use message::{ContentBlock, Message, Role, ToolDisplay, ToolStatus};
pub use scroll::{ScrollIndicator, ScrollPanel};
pub use shortcuts::ShortcutsBar;
pub use spinner::Spinner;
