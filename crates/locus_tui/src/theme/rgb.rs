//! RGB color for theme. Portable (u8) for TUI, CLI, or UI.

/// RGB triplet. Use with any terminal or UI color API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb(r, g, b)
    }

    pub fn r(self) -> u8 {
        self.0
    }
    pub fn g(self) -> u8 {
        self.1
    }
    pub fn b(self) -> u8 {
        self.2
    }

    /// Tuple for ratatui/crossterm: `(r, g, b)`.
    pub fn tuple(self) -> (u8, u8, u8) {
        (self.0, self.1, self.2)
    }
}

impl From<Rgb> for (u8, u8, u8) {
    fn from(c: Rgb) -> Self {
        c.tuple()
    }
}
