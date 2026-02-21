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

pub use chats::{ChatsLayout, chat_scroll_offset, CHAT_LEFT_INDENT, CHAT_MESSAGE_SPACING};
pub use head::{block_for_head, header_line, render_header, HeadLayout, HEADER_TITLE, HEADER_STATUS_READY};
pub use input::{block_for_input, block_for_input_bordered, InputLayout, INPUT_ICON, INPUT_PADDING_H};
pub use panel::{block_for_panel, PanelLayout};
pub use shortcut::{shortcut_inner_rect, shortcut_line};
pub use split::{
    main_splits, main_splits_with_padding, horizontal_split, vertical_split,
    MainSplits, HEADER_HEIGHT, FOOTER_HEIGHT,
};
pub use style::{
    rgb_to_color, border_style, background_style, border_focused_style,
    text_style, text_muted_style, success_style, danger_style, warning_style, info_style,
};
