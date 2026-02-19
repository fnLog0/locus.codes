//! Theme color constants for CLI and TUI.
//!
//! Colors inspired by Tokyo Night with locus branding adjustments.
//! Deep blacks with subtle blue undertones for a focused, technical aesthetic.
//! Defined as (R, G, B) tuples for use with any terminal color library.

/// Light theme (matches `:root` in oat-theme.css)
pub mod light {
    /// Background — #ffffff
    pub const BACKGROUND: (u8, u8, u8) = (255, 255, 255);
    /// Foreground text — #1a1b26
    pub const FOREGROUND: (u8, u8, u8) = (26, 27, 38);
    /// Card surface — #ffffff
    pub const CARD: (u8, u8, u8) = (255, 255, 255);
    /// Primary — #1a1b26
    pub const PRIMARY: (u8, u8, u8) = (26, 27, 38);
    /// Primary foreground — #ffffff
    pub const PRIMARY_FG: (u8, u8, u8) = (255, 255, 255);
    /// Secondary — #f4f4f5
    pub const SECONDARY: (u8, u8, u8) = (244, 244, 245);
    /// Muted — #f4f4f5
    pub const MUTED: (u8, u8, u8) = (244, 244, 245);
    /// Muted foreground — #565f89
    pub const MUTED_FG: (u8, u8, u8) = (86, 95, 137);
    /// Faint — #fafafa
    pub const FAINT: (u8, u8, u8) = (250, 250, 250);
    /// Accent — #7aa2f7 (Tokyo Night blue)
    pub const ACCENT: (u8, u8, u8) = (122, 162, 247);
    /// Danger — #f7768e (Tokyo Night red)
    pub const DANGER: (u8, u8, u8) = (247, 118, 142);
    /// Success — #9ece6a (Tokyo Night green)
    pub const SUCCESS: (u8, u8, u8) = (158, 206, 106);
    /// Warning — #e0af68 (Tokyo Night yellow)
    pub const WARNING: (u8, u8, u8) = (224, 175, 104);
    /// Border — #e5e5e5
    pub const BORDER: (u8, u8, u8) = (229, 229, 229);
    /// Input — #e5e5e5
    pub const INPUT: (u8, u8, u8) = (229, 229, 229);
    /// Ring / focus — #7aa2f7
    pub const RING: (u8, u8, u8) = (122, 162, 247);
    /// Code block background — #f8f8f8
    pub const CODE_BG: (u8, u8, u8) = (248, 248, 248);
    /// Tool block background — #f0f0f0
    pub const TOOL_BG: (u8, u8, u8) = (240, 240, 240);
    /// Bash command background — #f5f5f5
    pub const BASH_BG: (u8, u8, u8) = (245, 245, 245);
    /// Thinking block background — #fafafa
    pub const THINK_BG: (u8, u8, u8) = (250, 250, 250);
    /// File path color — #7aa2f7 (accent blue)
    pub const FILE_PATH: (u8, u8, u8) = (122, 162, 247);
    /// Tool name color — #565f89
    pub const TOOL_NAME: (u8, u8, u8) = (86, 95, 137);
    /// Timestamp color — #9aa5ce
    pub const TIMESTAMP: (u8, u8, u8) = (154, 165, 206);
    /// Info color — #7dcfff (Tokyo Night cyan)
    pub const INFO: (u8, u8, u8) = (125, 207, 255);
}

/// Dark theme (Tokyo Night inspired with locus branding)
///
/// Color palette:
/// - Backgrounds: Deep blacks (#0a0a0f to #1f1f2e) with blue undertones
/// - Foregrounds: Soft blue-white (#c0caf5) for readability
/// - Accents: Tokyo Night blues (#7aa2f7), greens (#9ece6a), reds (#f7768e)
pub mod dark {
    /// Background — #0a0a0f (deep black with blue tint)
    pub const BACKGROUND: (u8, u8, u8) = (10, 10, 15);
    /// Foreground text — #c0caf5 (Tokyo Night foreground)
    pub const FOREGROUND: (u8, u8, u8) = (192, 202, 245);
    /// Card surface — #16161e
    pub const CARD: (u8, u8, u8) = (22, 22, 30);
    /// Primary — #c0caf5
    pub const PRIMARY: (u8, u8, u8) = (192, 202, 245);
    /// Primary foreground — #0a0a0f
    pub const PRIMARY_FG: (u8, u8, u8) = (10, 10, 15);
    /// Secondary — #1f1f2e
    pub const SECONDARY: (u8, u8, u8) = (31, 31, 46);
    /// Muted — #1f1f2e
    pub const MUTED: (u8, u8, u8) = (31, 31, 46);
    /// Muted foreground — #565f89 (Tokyo Night comment gray)
    pub const MUTED_FG: (u8, u8, u8) = (86, 95, 137);
    /// Faint — #13111a
    pub const FAINT: (u8, u8, u8) = (19, 17, 26);
    /// Accent — #7aa2f7 (Tokyo Night blue)
    pub const ACCENT: (u8, u8, u8) = (122, 162, 247);
    /// Danger — #f7768e (Tokyo Night red)
    pub const DANGER: (u8, u8, u8) = (247, 118, 142);
    /// Success — #9ece6a (Tokyo Night green)
    pub const SUCCESS: (u8, u8, u8) = (158, 206, 106);
    /// Warning — #e0af68 (Tokyo Night yellow)
    pub const WARNING: (u8, u8, u8) = (224, 175, 104);
    /// Border — #1f1f2e
    pub const BORDER: (u8, u8, u8) = (31, 31, 46);
    /// Input — #1a1a26
    pub const INPUT: (u8, u8, u8) = (26, 26, 38);
    /// Ring / focus — #7aa2f7
    pub const RING: (u8, u8, u8) = (122, 162, 247);
    /// Code block background — #16161e
    pub const CODE_BG: (u8, u8, u8) = (22, 22, 30);
    /// Tool block background — #11111a
    pub const TOOL_BG: (u8, u8, u8) = (17, 17, 26);
    /// Bash command background — #1a1b26 (Tokyo Night bg)
    pub const BASH_BG: (u8, u8, u8) = (26, 27, 38);
    /// Thinking block background — #0f0f18
    pub const THINK_BG: (u8, u8, u8) = (15, 15, 24);
    /// File path color — #7aa2f7 (accent blue)
    pub const FILE_PATH: (u8, u8, u8) = (122, 162, 247);
    /// Tool name color — #565f89
    pub const TOOL_NAME: (u8, u8, u8) = (86, 95, 137);
    /// Timestamp color — #3d4166
    pub const TIMESTAMP: (u8, u8, u8) = (61, 65, 102);
    /// Info color — #7dcfff (Tokyo Night cyan)
    pub const INFO: (u8, u8, u8) = (125, 207, 255);
    /// Purple accent — #bb9af7 (Tokyo Night purple)
    pub const PURPLE: (u8, u8, u8) = (187, 154, 247);
}
