//! Locus palette: semantic color roles (surfaces, borders, text, elements, chrome).
//!
//! Structure follows the same roles as in the reference theme (zed_default_theme):
//! background, surface, elevated surface; borders; element and ghost-element states;
//! text and icon levels; semantic (accent, danger, success, warning, info); UI chrome.

use super::Appearance;
use super::rgb::Rgb;

/// One full palette for an appearance (dark or light). All colors are semantic roles.
#[derive(Clone, Debug, PartialEq)]
pub struct LocusPalette {
    // --- Surfaces
    /// App / window background.
    pub background: Rgb,
    /// Panel, card, tab area.
    pub surface_background: Rgb,
    /// Popover, menu, dialog.
    pub elevated_surface_background: Rgb,

    // --- Borders
    pub border: Rgb,
    pub border_variant: Rgb,
    pub border_focused: Rgb,
    pub border_selected: Rgb,
    pub border_disabled: Rgb,

    // --- Elements
    pub element_background: Rgb,
    pub element_hover: Rgb,
    pub element_active: Rgb,
    pub element_selected: Rgb,
    pub element_disabled: Rgb,
    pub ghost_element_background: Rgb,
    pub ghost_element_hover: Rgb,
    pub ghost_element_selected: Rgb,
    pub ghost_element_disabled: Rgb,

    // --- Text
    pub text: Rgb,
    pub text_muted: Rgb,
    pub text_placeholder: Rgb,
    pub text_disabled: Rgb,
    pub text_accent: Rgb,

    // --- Icons
    pub icon: Rgb,
    pub icon_muted: Rgb,
    pub icon_disabled: Rgb,
    pub icon_accent: Rgb,

    // --- Semantic
    pub accent: Rgb,
    pub danger: Rgb,
    pub success: Rgb,
    pub warning: Rgb,
    pub info: Rgb,

    // --- UI chrome
    pub status_bar_background: Rgb,
    pub tab_bar_background: Rgb,
    pub tab_inactive_background: Rgb,
    pub tab_active_background: Rgb,
    pub panel_background: Rgb,
    pub panel_focused_border: Rgb,
    pub scrollbar_thumb_background: Rgb,
    pub scrollbar_thumb_hover_background: Rgb,
    pub scrollbar_thumb_active: Rgb,
    pub scrollbar_track_background: Rgb,
    pub pane_focused_border: Rgb,

    // --- Editor / code (for future use)
    pub editor_background: Rgb,
    pub editor_foreground: Rgb,
    pub editor_line_number: Rgb,
}

impl LocusPalette {
    /// Default Locus dark palette (refined: deeper blacks, softer accents).
    pub fn locus_dark() -> Self {
        Self {
            background: Rgb(8, 8, 12),
            surface_background: Rgb(16, 17, 24),
            elevated_surface_background: Rgb(22, 23, 32),
            border: Rgb(28, 30, 42),
            border_variant: Rgb(26, 26, 38),
            border_focused: Rgb(99, 148, 255),
            border_selected: Rgb(99, 148, 255),
            border_disabled: Rgb(61, 65, 102),
            element_background: Rgb(26, 27, 38),
            element_hover: Rgb(36, 40, 59),
            element_active: Rgb(36, 40, 59),
            element_selected: Rgb(36, 40, 59),
            element_disabled: Rgb(26, 27, 38),
            ghost_element_background: Rgb(0, 0, 0),
            ghost_element_hover: Rgb(31, 31, 46),
            ghost_element_selected: Rgb(36, 40, 59),
            ghost_element_disabled: Rgb(26, 26, 38),
            text: Rgb(200, 210, 245),
            text_muted: Rgb(70, 78, 110),
            text_placeholder: Rgb(70, 78, 110),
            text_disabled: Rgb(61, 65, 102),
            text_accent: Rgb(99, 148, 255),
            icon: Rgb(200, 210, 245),
            icon_muted: Rgb(70, 78, 110),
            icon_disabled: Rgb(61, 65, 102),
            icon_accent: Rgb(99, 148, 255),
            accent: Rgb(99, 148, 255),
            danger: Rgb(255, 100, 120),
            success: Rgb(120, 220, 120),
            warning: Rgb(240, 185, 100),
            info: Rgb(100, 200, 255),
            status_bar_background: Rgb(16, 17, 24),
            tab_bar_background: Rgb(16, 17, 24),
            tab_inactive_background: Rgb(16, 17, 24),
            tab_active_background: Rgb(8, 8, 12),
            panel_background: Rgb(16, 17, 24),
            panel_focused_border: Rgb(99, 148, 255),
            scrollbar_thumb_background: Rgb(61, 65, 102),
            scrollbar_thumb_hover_background: Rgb(86, 95, 137),
            scrollbar_thumb_active: Rgb(100, 110, 150),
            scrollbar_track_background: Rgb(17, 17, 26),
            pane_focused_border: Rgb(99, 148, 255),
            editor_background: Rgb(8, 8, 12),
            editor_foreground: Rgb(200, 210, 245),
            editor_line_number: Rgb(70, 78, 110),
        }
    }

    /// Default Locus light palette.
    pub fn locus_light() -> Self {
        Self {
            background: Rgb(255, 255, 255),
            surface_background: Rgb(255, 255, 255),
            elevated_surface_background: Rgb(248, 248, 248),
            border: Rgb(229, 229, 229),
            border_variant: Rgb(244, 244, 245),
            border_focused: Rgb(122, 162, 247),
            border_selected: Rgb(122, 162, 247),
            border_disabled: Rgb(203, 213, 225),
            element_background: Rgb(244, 244, 245),
            element_hover: Rgb(229, 229, 229),
            element_active: Rgb(229, 229, 229),
            element_selected: Rgb(229, 229, 229),
            element_disabled: Rgb(244, 244, 245),
            ghost_element_background: Rgb(0, 0, 0),
            ghost_element_hover: Rgb(244, 244, 245),
            ghost_element_selected: Rgb(229, 229, 229),
            ghost_element_disabled: Rgb(244, 244, 245),
            text: Rgb(26, 27, 38),
            text_muted: Rgb(86, 95, 137),
            text_placeholder: Rgb(86, 95, 137),
            text_disabled: Rgb(161, 161, 170),
            text_accent: Rgb(122, 162, 247),
            icon: Rgb(26, 27, 38),
            icon_muted: Rgb(86, 95, 137),
            icon_disabled: Rgb(161, 161, 170),
            icon_accent: Rgb(122, 162, 247),
            accent: Rgb(122, 162, 247),
            danger: Rgb(247, 118, 142),
            success: Rgb(158, 206, 106),
            warning: Rgb(224, 175, 104),
            info: Rgb(125, 207, 255),
            status_bar_background: Rgb(255, 255, 255),
            tab_bar_background: Rgb(255, 255, 255),
            tab_inactive_background: Rgb(244, 244, 245),
            tab_active_background: Rgb(255, 255, 255),
            panel_background: Rgb(255, 255, 255),
            panel_focused_border: Rgb(122, 162, 247),
            scrollbar_thumb_background: Rgb(203, 213, 225),
            scrollbar_thumb_hover_background: Rgb(161, 161, 170),
            scrollbar_thumb_active: Rgb(180, 190, 210),
            scrollbar_track_background: Rgb(248, 248, 248),
            pane_focused_border: Rgb(122, 162, 247),
            editor_background: Rgb(255, 255, 255),
            editor_foreground: Rgb(26, 27, 38),
            editor_line_number: Rgb(86, 95, 137),
        }
    }

    /// Palette for the given appearance.
    pub fn for_appearance(appearance: Appearance) -> Self {
        match appearance {
            Appearance::Dark => Self::locus_dark(),
            Appearance::Light => Self::locus_light(),
        }
    }
}
