//! Theme color constants for CLI and TUI.
//!
//! Colors sourced from `apps/landing/src/oat-theme.css`.
//! Defined as (R, G, B) tuples for use with any terminal color library.

/// Light theme (matches `:root` in oat-theme.css)
pub mod light {
    /// Background — #ffffff
    pub const BACKGROUND: (u8, u8, u8) = (255, 255, 255);
    /// Foreground text — #000000
    pub const FOREGROUND: (u8, u8, u8) = (0, 0, 0);
    /// Card surface — #ffffff
    pub const CARD: (u8, u8, u8) = (255, 255, 255);
    /// Primary — #000000
    pub const PRIMARY: (u8, u8, u8) = (0, 0, 0);
    /// Primary foreground — #ffffff
    pub const PRIMARY_FG: (u8, u8, u8) = (255, 255, 255);
    /// Secondary — #f4f4f5
    pub const SECONDARY: (u8, u8, u8) = (244, 244, 245);
    /// Muted — #f4f4f5
    pub const MUTED: (u8, u8, u8) = (244, 244, 245);
    /// Muted foreground — #666666
    pub const MUTED_FG: (u8, u8, u8) = (102, 102, 102);
    /// Faint — #fafafa
    pub const FAINT: (u8, u8, u8) = (250, 250, 250);
    /// Accent — #f4f4f5
    pub const ACCENT: (u8, u8, u8) = (244, 244, 245);
    /// Danger — #df514c
    pub const DANGER: (u8, u8, u8) = (223, 81, 76);
    /// Success — #4caf50
    pub const SUCCESS: (u8, u8, u8) = (76, 175, 80);
    /// Warning — #ff8c00
    pub const WARNING: (u8, u8, u8) = (255, 140, 0);
    /// Border — #e5e5e5
    pub const BORDER: (u8, u8, u8) = (229, 229, 229);
    /// Input — #e5e5e5
    pub const INPUT: (u8, u8, u8) = (229, 229, 229);
    /// Ring / focus — #000000
    pub const RING: (u8, u8, u8) = (0, 0, 0);
}

/// Dark theme (matches `[data-theme="dark"]` in oat-theme.css)
pub mod dark {
    /// Background — #0a0a0a
    pub const BACKGROUND: (u8, u8, u8) = (10, 10, 10);
    /// Foreground text — #ffffff
    pub const FOREGROUND: (u8, u8, u8) = (255, 255, 255);
    /// Card surface — #18181b
    pub const CARD: (u8, u8, u8) = (24, 24, 27);
    /// Primary — #ffffff
    pub const PRIMARY: (u8, u8, u8) = (255, 255, 255);
    /// Primary foreground — #0a0a0a
    pub const PRIMARY_FG: (u8, u8, u8) = (10, 10, 10);
    /// Secondary — #262626
    pub const SECONDARY: (u8, u8, u8) = (38, 38, 38);
    /// Muted — #262626
    pub const MUTED: (u8, u8, u8) = (38, 38, 38);
    /// Muted foreground — #888888
    pub const MUTED_FG: (u8, u8, u8) = (136, 136, 136);
    /// Faint — #141414
    pub const FAINT: (u8, u8, u8) = (20, 20, 20);
    /// Accent — #262626
    pub const ACCENT: (u8, u8, u8) = (38, 38, 38);
    /// Danger — #df514c
    pub const DANGER: (u8, u8, u8) = (223, 81, 76);
    /// Success — #4caf50
    pub const SUCCESS: (u8, u8, u8) = (76, 175, 80);
    /// Warning — #ff8c00
    pub const WARNING: (u8, u8, u8) = (255, 140, 0);
    /// Border — #262626
    pub const BORDER: (u8, u8, u8) = (38, 38, 38);
    /// Input — #262626
    pub const INPUT: (u8, u8, u8) = (38, 38, 38);
    /// Ring / focus — #ffffff
    pub const RING: (u8, u8, u8) = (255, 255, 255);
}
