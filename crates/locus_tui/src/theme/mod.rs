//! Locus theme: semantic color palette for locus TUI.
//!
//! Structure is based on the same semantic roles as the reference theme in
//! `zed_default_theme` (surfaces, borders, text, elements, chrome). Use this
//! palette when building the TUI.
//!
//! # Example
//!
//! ```ignore
//! use locus_tui::theme::{Appearance, LocusPalette};
//!
//! let palette = LocusPalette::locus_dark();
//! let text = palette.text.tuple(); // (r, g, b) for ratatui
//!
//! let palette = LocusPalette::for_appearance(Appearance::Light);
//! ```

mod appearance;
mod palette;
mod rgb;

pub use appearance::Appearance;
pub use palette::LocusPalette;
pub use rgb::Rgb;
