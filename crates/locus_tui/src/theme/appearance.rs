//! Theme appearance: light or dark. No separate foreground/background buckets.

/// Whether the theme is light or dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    #[default]
    Dark,
    Light,
}

impl Appearance {
    pub fn is_dark(self) -> bool {
        matches!(self, Appearance::Dark)
    }
    pub fn is_light(self) -> bool {
        matches!(self, Appearance::Light)
    }
}
