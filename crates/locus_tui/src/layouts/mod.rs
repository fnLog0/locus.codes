//! Layout components built from [crate::utils] and [crate::theme].
//!
//! - **[split]** — Split the screen into header, body, footer or left/right.
//! - **[panel]** — Bordered or plain panel with inner padded rect and theme-backed block.
//! - **[style]** — Map palette [Rgb] to ratatui [Style]/[Color] for borders and text.
//! - **[head]** — Header strip layout and styled header line.
//! - **[chats]** — Chat area layout and scroll/indent helpers.
//! - **[input]** — Input bar layout and block.
//! - **[shortcut]** — Shortcut hint line (below input).

mod chats;
mod head;
mod input;
mod panel;
mod shortcut;
mod split;
mod style;

pub use chats::{CHAT_LEFT_INDENT, CHAT_MESSAGE_SPACING, ChatsLayout, chat_scroll_offset};
pub use head::{
    HEADER_STATUS_READY, HEADER_TAGLINE, HEADER_TITLE, HeadLayout, block_for_head,
    header_status_line, header_title_line, render_header,
};
pub use input::{
    INPUT_ICON, INPUT_PADDING_H, InputLayout, block_for_input, block_for_input_bordered,
};
pub use panel::{PanelLayout, block_for_panel};
pub use shortcut::{shortcut_inner_rect, shortcut_line};
pub use split::{
    FOOTER_HEIGHT, HEADER_HEIGHT, MainSplits, horizontal_split, main_splits,
    main_splits_with_footer_height, main_splits_with_padding,
    main_splits_with_padding_and_footer_height, vertical_split,
};
pub use style::{
    accent_style, background_style, border_focused_style, border_style, danger_style, info_style,
    rgb_to_color, success_style, text_muted_style, text_style, warning_style,
};
