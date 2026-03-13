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
            background: Rgb(7, 8, 11),
            surface_background: Rgb(15, 17, 23),
            elevated_surface_background: Rgb(21, 24, 32),
            border: Rgb(34, 38, 50),
            border_variant: Rgb(27, 30, 40),
            border_focused: Rgb(103, 155, 255),
            border_selected: Rgb(124, 174, 255),
            border_disabled: Rgb(58, 63, 82),
            element_background: Rgb(22, 24, 31),
            element_hover: Rgb(31, 35, 46),
            element_active: Rgb(37, 41, 54),
            element_selected: Rgb(34, 39, 54),
            element_disabled: Rgb(20, 22, 28),
            ghost_element_background: Rgb(7, 8, 11),
            ghost_element_hover: Rgb(23, 26, 35),
            ghost_element_selected: Rgb(28, 33, 44),
            ghost_element_disabled: Rgb(16, 18, 24),
            text: Rgb(214, 220, 238),
            text_muted: Rgb(104, 114, 145),
            text_placeholder: Rgb(84, 93, 122),
            text_disabled: Rgb(63, 69, 91),
            text_accent: Rgb(124, 174, 255),
            icon: Rgb(190, 198, 222),
            icon_muted: Rgb(104, 114, 145),
            icon_disabled: Rgb(63, 69, 91),
            icon_accent: Rgb(124, 174, 255),
            accent: Rgb(103, 155, 255),
            danger: Rgb(255, 110, 128),
            success: Rgb(126, 224, 158),
            warning: Rgb(244, 188, 108),
            info: Rgb(102, 208, 255),
            status_bar_background: Rgb(13, 15, 21),
            tab_bar_background: Rgb(13, 15, 21),
            tab_inactive_background: Rgb(15, 17, 23),
            tab_active_background: Rgb(9, 10, 14),
            panel_background: Rgb(15, 17, 23),
            panel_focused_border: Rgb(103, 155, 255),
            scrollbar_thumb_background: Rgb(68, 76, 102),
            scrollbar_thumb_hover_background: Rgb(92, 102, 132),
            scrollbar_thumb_active: Rgb(116, 128, 162),
            scrollbar_track_background: Rgb(11, 12, 17),
            pane_focused_border: Rgb(103, 155, 255),
            editor_background: Rgb(10, 11, 16),
            editor_foreground: Rgb(214, 220, 238),
            editor_line_number: Rgb(92, 101, 130),
        }
    }

    /// Default Locus light palette.
    pub fn locus_light() -> Self {
        Self {
            background: Rgb(245, 247, 251),
            surface_background: Rgb(255, 255, 255),
            elevated_surface_background: Rgb(237, 241, 248),
            border: Rgb(208, 216, 230),
            border_variant: Rgb(224, 230, 240),
            border_focused: Rgb(70, 116, 210),
            border_selected: Rgb(90, 136, 230),
            border_disabled: Rgb(220, 225, 236),
            element_background: Rgb(238, 242, 248),
            element_hover: Rgb(228, 234, 244),
            element_active: Rgb(218, 226, 239),
            element_selected: Rgb(222, 230, 243),
            element_disabled: Rgb(241, 244, 249),
            ghost_element_background: Rgb(245, 247, 251),
            ghost_element_hover: Rgb(235, 240, 247),
            ghost_element_selected: Rgb(228, 234, 244),
            ghost_element_disabled: Rgb(242, 245, 250),
            text: Rgb(26, 33, 48),
            text_muted: Rgb(96, 107, 131),
            text_placeholder: Rgb(124, 134, 157),
            text_disabled: Rgb(160, 170, 191),
            text_accent: Rgb(70, 116, 210),
            icon: Rgb(49, 59, 79),
            icon_muted: Rgb(106, 116, 139),
            icon_disabled: Rgb(160, 170, 191),
            icon_accent: Rgb(70, 116, 210),
            accent: Rgb(70, 116, 210),
            danger: Rgb(214, 92, 110),
            success: Rgb(67, 153, 100),
            warning: Rgb(196, 137, 55),
            info: Rgb(45, 146, 204),
            status_bar_background: Rgb(239, 243, 249),
            tab_bar_background: Rgb(239, 243, 249),
            tab_inactive_background: Rgb(234, 239, 247),
            tab_active_background: Rgb(255, 255, 255),
            panel_background: Rgb(255, 255, 255),
            panel_focused_border: Rgb(70, 116, 210),
            scrollbar_thumb_background: Rgb(173, 184, 204),
            scrollbar_thumb_hover_background: Rgb(146, 159, 183),
            scrollbar_thumb_active: Rgb(119, 135, 164),
            scrollbar_track_background: Rgb(237, 241, 248),
            pane_focused_border: Rgb(70, 116, 210),
            editor_background: Rgb(250, 251, 253),
            editor_foreground: Rgb(26, 33, 48),
            editor_line_number: Rgb(112, 122, 145),
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
