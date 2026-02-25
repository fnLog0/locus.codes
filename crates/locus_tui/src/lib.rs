//! locus-tui — TUI for locus.codes.
//!
//! Theming in `theme`; layout in `layouts`; messages in `messages`; state and view in [state] and [view].
//! Run with [run_tui].

pub mod animation;
pub mod diff;
pub mod layouts;
pub mod messages;
pub mod run;
pub mod runtime_events;
pub mod state;
pub mod theme;
pub mod utils;
pub mod view;
pub mod web_automation;

pub use run::{run_tui, run_tui_with_runtime};
pub use state::{ChatItem, Screen, TuiState};
pub use view::draw as draw_view;
