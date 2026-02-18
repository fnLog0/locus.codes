//! locus-ui — terminal UI for locus.codes (Ratatui + Crossterm).
//!
//! Provides loading screen, components, and animation utilities.

pub mod animation;
pub mod components;

pub use animation::Shimmer;
pub use components::{Grid, Loader, Spinner};
