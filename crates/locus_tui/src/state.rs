//! TUI state: chat items, input buffer, scroll, theme.
//!
//! [TuiState] holds everything the view needs to render. [ChatItem] wraps
//! message types from [crate::messages] so we can store a single list.

use crate::messages::{
    meta_tool::MetaToolMessage,
    tool::ToolCallMessage,
    user::UserMessage,
    ai_message::AiMessage,
    ai_think_message::AiThinkMessage,
    error::ErrorMessage,
};
use crate::theme::{Appearance, LocusPalette};

/// Which screen is currently shown (main chat vs debug traces).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Main,
    DebugTraces,
}

/// Max trace lines to keep (older lines dropped).
const MAX_TRACE_LINES: usize = 2000;

/// One item in the chat: user, assistant, thinking, tool, tool group, meta-tool, or error.
#[derive(Debug, Clone)]
pub enum ChatItem {
    User(UserMessage),
    Ai(AiMessage),
    Think(AiThinkMessage),
    Tool(ToolCallMessage),
    ToolGroup(Vec<ToolCallMessage>),
    MetaTool(MetaToolMessage),
    Error(ErrorMessage),
    Separator(String),
}

/// TUI application state.
#[derive(Debug)]
pub struct TuiState {
    /// Ordered list of chat items to display.
    pub messages: Vec<ChatItem>,
    /// Current input line (footer).
    pub input_buffer: String,
    /// Cursor position within input_buffer (0..=len).
    pub input_cursor: usize,
    /// Vertical scroll offset (number of lines scrolled up).
    pub scroll: usize,
    /// When true, keep scroll at bottom on new content; when false, user scrolled up.
    pub auto_scroll: bool,
    /// Theme palette (dark/light).
    pub palette: LocusPalette,
    /// Optional status text for header right side.
    pub status: String,
    /// Accumulated assistant text for current turn (pushed on TurnEnd).
    pub current_ai_text: String,
    /// Accumulated thinking text for current turn (pushed on TurnEnd).
    pub current_think_text: String,
    /// True from TurnStart(Assistant) until TurnEnd (streaming in progress).
    pub is_streaming: bool,
    /// Incremented each run_loop iteration for cursor blink / animations.
    pub frame_count: u64,
    /// When true, next draw should run; cleared after draw. Redraw on any state change.
    pub needs_redraw: bool,
    /// Cached line list; invalidated by push_* / flush_turn / resize.
    pub cached_lines: Vec<ratatui::text::Line<'static>>,
    /// True when cached_lines is stale.
    pub cache_dirty: bool,
    /// Last content height from previous draw (for scroll clamp).
    pub last_content_height: usize,
    /// Last viewport height from previous draw (for scroll clamp).
    pub last_viewport_height: usize,
    /// When set, status is transient and should auto-clear after duration.
    pub status_set_at: Option<std::time::Instant>,
    /// Never auto-clear status (e.g. "Session ended").
    pub status_permanent: bool,
    /// Shimmer for running tool name animation (ticked when any tool is running).
    pub tool_shimmer: Option<crate::animation::Shimmer>,
    /// Current screen (main chat or debug traces).
    pub screen: Screen,
    /// Debug trace lines (session events, etc.). Newest at end.
    pub trace_lines: Vec<String>,
    /// Scroll offset for debug trace view (lines scrolled up).
    pub trace_scroll: usize,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input_buffer: String::new(),
            input_cursor: 0,
            scroll: 0,
            auto_scroll: true,
            palette: LocusPalette::locus_dark(),
            status: String::new(),
            current_ai_text: String::new(),
            current_think_text: String::new(),
            is_streaming: false,
            frame_count: 0,
            needs_redraw: true,
            cached_lines: Vec::new(),
            cache_dirty: true,
            last_content_height: 0,
            last_viewport_height: 0,
            status_set_at: None,
            status_permanent: false,
            tool_shimmer: None,
            screen: Screen::Main,
            trace_lines: Vec::new(),
            trace_scroll: 0,
        }
    }
}

impl TuiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_appearance(appearance: Appearance) -> Self {
        Self {
            palette: LocusPalette::for_appearance(appearance),
            ..Self::default()
        }
    }

    /// Push a user message.
    pub fn push_user(&mut self, text: String, timestamp: Option<String>) {
        self.messages.push(ChatItem::User(UserMessage { text, timestamp }));
        self.cache_dirty = true;
        self.needs_redraw = true;
        if self.auto_scroll {
            self.scroll = 0;
        }
    }

    /// Push an AI message.
    pub fn push_ai(&mut self, text: String, timestamp: Option<String>) {
        self.messages.push(ChatItem::Ai(AiMessage { text, timestamp }));
        self.cache_dirty = true;
        self.needs_redraw = true;
        if self.auto_scroll {
            self.scroll = 0;
        }
    }

    /// Push a thinking message.
    pub fn push_think(&mut self, text: String, collapsed: bool) {
        self.messages.push(ChatItem::Think(AiThinkMessage { text, collapsed }));
        self.cache_dirty = true;
        self.needs_redraw = true;
        if self.auto_scroll {
            self.scroll = 0;
        }
    }

    /// Push or update a tool call (caller can push Running then replace with Done/Error).
    pub fn push_tool(&mut self, msg: ToolCallMessage) {
        self.messages.push(ChatItem::Tool(msg));
        self.cache_dirty = true;
        self.needs_redraw = true;
        if self.auto_scroll {
            self.scroll = 0;
        }
    }

    /// Push a tool call, grouping with the last ToolGroup if one is active.
    pub fn push_tool_grouped(&mut self, msg: ToolCallMessage) {
        match self.messages.last_mut() {
            Some(ChatItem::ToolGroup(group)) => {
                group.push(msg);
            }
            Some(ChatItem::Tool(existing)) => {
                // Second consecutive tool — upgrade to ToolGroup
                let first = std::mem::replace(existing, ToolCallMessage::running("", "", None));
                let idx = self.messages.len() - 1;
                self.messages[idx] = ChatItem::ToolGroup(vec![first, msg]);
            }
            _ => {
                // First tool in a potential group — push as single Tool
                self.messages.push(ChatItem::Tool(msg));
            }
        }
        self.cache_dirty = true;
        self.needs_redraw = true;
        if self.auto_scroll {
            self.scroll = 0;
        }
    }

    /// Find a tool by id and update it (for ToolDone matching by tool_use_id).
    pub fn update_tool_by_id(&mut self, tool_use_id: &str, duration_ms: u64, success: bool) -> bool {
        for item in self.messages.iter_mut().rev() {
            match item {
                ChatItem::Tool(t) if t.id.as_deref() == Some(tool_use_id) => {
                    *t = ToolCallMessage::done(
                        t.id.clone(),
                        t.tool_name.clone(),
                        duration_ms,
                        success,
                        t.summary.clone(),
                    );
                    self.cache_dirty = true;
                    self.needs_redraw = true;
                    return true;
                }
                ChatItem::ToolGroup(group) => {
                    for t in group.iter_mut() {
                        if t.id.as_deref() == Some(tool_use_id) {
                            *t = ToolCallMessage::done(
                                t.id.clone(),
                                t.tool_name.clone(),
                                duration_ms,
                                success,
                                t.summary.clone(),
                            );
                            self.cache_dirty = true;
                            self.needs_redraw = true;
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Push a meta-tool message.
    pub fn push_meta_tool(&mut self, msg: MetaToolMessage) {
        self.messages.push(ChatItem::MetaTool(msg));
        self.cache_dirty = true;
        self.needs_redraw = true;
        if self.auto_scroll {
            self.scroll = 0;
        }
    }

    /// Push an inline error message.
    pub fn push_error(&mut self, text: String, timestamp: Option<String>) {
        self.messages.push(ChatItem::Error(ErrorMessage { text, timestamp }));
        self.cache_dirty = true;
        self.needs_redraw = true;
        if self.auto_scroll {
            self.scroll = 0;
        }
    }

    /// Push a session separator (e.g. "New session").
    pub fn push_separator(&mut self, label: String) {
        self.messages.push(ChatItem::Separator(label));
        self.cache_dirty = true;
        self.needs_redraw = true;
        if self.auto_scroll {
            self.scroll = 0;
        }
    }

    /// Input buffer: insert character at cursor.
    pub fn input_insert(&mut self, c: char) {
        self.input_buffer.insert(self.input_cursor, c);
        self.input_cursor += c.len_utf8();
        self.needs_redraw = true;
    }

    /// Input buffer: delete character before cursor (UTF-8 safe).
    pub fn input_backspace(&mut self) {
        if self.input_cursor == 0 {
            return;
        }
        let mut start = self.input_cursor - 1;
        while start > 0 && (self.input_buffer.as_bytes()[start] & 0xC0) == 0x80 {
            start -= 1;
        }
        self.input_buffer.drain(start..self.input_cursor);
        self.input_cursor = start;
        self.needs_redraw = true;
    }

    /// Input buffer: delete character at cursor (forward delete, UTF-8 safe).
    pub fn input_delete(&mut self) {
        if self.input_cursor >= self.input_buffer.len() {
            return;
        }
        let mut end = self.input_cursor + 1;
        while end < self.input_buffer.len() && (self.input_buffer.as_bytes()[end] & 0xC0) == 0x80 {
            end += 1;
        }
        self.input_buffer.drain(self.input_cursor..end);
        self.needs_redraw = true;
    }

    /// Move cursor left one character (UTF-8 safe).
    pub fn input_cursor_left(&mut self) {
        if self.input_cursor == 0 {
            return;
        }
        let mut start = self.input_cursor - 1;
        while start > 0 && (self.input_buffer.as_bytes()[start] & 0xC0) == 0x80 {
            start -= 1;
        }
        self.input_cursor = start;
        self.needs_redraw = true;
    }

    /// Move cursor right one character (UTF-8 safe).
    pub fn input_cursor_right(&mut self) {
        if self.input_cursor >= self.input_buffer.len() {
            return;
        }
        let mut end = self.input_cursor + 1;
        while end < self.input_buffer.len() && (self.input_buffer.as_bytes()[end] & 0xC0) == 0x80 {
            end += 1;
        }
        self.input_cursor = end;
        self.needs_redraw = true;
    }

    /// Cursor to start of input.
    pub fn input_cursor_home(&mut self) {
        self.input_cursor = 0;
        self.needs_redraw = true;
    }

    /// Cursor to end of input; if empty, enable auto_scroll and scroll to bottom.
    pub fn input_cursor_end(&mut self) {
        self.input_cursor = self.input_buffer.len();
        if self.input_buffer.is_empty() {
            self.auto_scroll = true;
            self.scroll = 0;
        }
        self.needs_redraw = true;
    }

    /// Clear entire input buffer (Ctrl+U).
    pub fn input_clear_line(&mut self) {
        self.input_buffer.clear();
        self.input_cursor = 0;
        self.needs_redraw = true;
    }

    /// Delete from cursor to end of line (Ctrl+K).
    pub fn input_kill_to_end(&mut self) {
        self.input_buffer.truncate(self.input_cursor);
        self.needs_redraw = true;
    }

    /// Input buffer: clear and return current line (for submit).
    pub fn input_take(&mut self) -> String {
        let line = std::mem::take(&mut self.input_buffer);
        self.input_cursor = 0;
        self.needs_redraw = true;
        line
    }

    /// Scroll up (increase offset); disables auto_scroll.
    pub fn scroll_up(&mut self, delta: usize) {
        self.auto_scroll = false;
        self.scroll = self.scroll.saturating_add(delta);
        self.needs_redraw = true;
    }

    /// Scroll down (decrease offset); re-enables auto_scroll when at bottom.
    pub fn scroll_down(&mut self, delta: usize) {
        self.scroll = self.scroll.saturating_sub(delta);
        if self.scroll == 0 {
            self.auto_scroll = true;
        }
        self.needs_redraw = true;
    }

    /// Append a line to the debug trace buffer (for Ctrl+D debug screen). Drops oldest if over capacity.
    pub fn push_trace_line(&mut self, line: String) {
        self.trace_lines.push(line);
        if self.trace_lines.len() > MAX_TRACE_LINES {
            self.trace_lines.drain(0..self.trace_lines.len() - MAX_TRACE_LINES);
        }
        self.needs_redraw = true;
    }

    /// Scroll the trace view up.
    pub fn trace_scroll_up(&mut self, delta: usize) {
        self.trace_scroll = self.trace_scroll.saturating_add(delta);
        self.needs_redraw = true;
    }

    /// Scroll the trace view down.
    pub fn trace_scroll_down(&mut self, delta: usize) {
        self.trace_scroll = self.trace_scroll.saturating_sub(delta);
        self.needs_redraw = true;
    }

    /// Flush accumulated assistant/thinking text into messages (call on TurnEnd).
    pub fn flush_turn(&mut self) {
        let think = std::mem::take(&mut self.current_think_text);
        if !think.is_empty() {
            self.push_think(think, false);
        }
        let ai = std::mem::take(&mut self.current_ai_text);
        if !ai.is_empty() {
            let ts = chrono::Local::now().format("%H:%M").to_string();
            self.push_ai(ai, Some(ts));
        }
        if self.auto_scroll {
            self.scroll = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::tool::ToolCallMessage;

    #[test]
    fn input_insert_ascii() {
        let mut s = TuiState::new();
        s.input_insert('a');
        s.input_insert('b');
        assert_eq!(s.input_buffer, "ab");
        assert_eq!(s.input_cursor, 2);
    }

    #[test]
    fn input_insert_utf8_emoji() {
        let mut s = TuiState::new();
        s.input_insert('é');
        s.input_insert('🎉');
        assert_eq!(s.input_buffer, "é🎉");
        assert_eq!(s.input_cursor, "é🎉".len());
    }

    #[test]
    fn input_backspace_at_end() {
        let mut s = TuiState::new();
        s.input_buffer = "hi".to_string();
        s.input_cursor = 2;
        s.input_backspace();
        assert_eq!(s.input_buffer, "h");
        assert_eq!(s.input_cursor, 1);
    }

    #[test]
    fn input_backspace_at_zero_no_op() {
        let mut s = TuiState::new();
        s.input_buffer = "x".to_string();
        s.input_cursor = 0;
        s.input_backspace();
        assert_eq!(s.input_buffer, "x");
    }

    #[test]
    fn input_take_returns_and_resets() {
        let mut s = TuiState::new();
        s.input_buffer = "hello".to_string();
        s.input_cursor = 5;
        let line = s.input_take();
        assert_eq!(line, "hello");
        assert!(s.input_buffer.is_empty());
        assert_eq!(s.input_cursor, 0);
    }

    #[test]
    fn flush_turn_pushes_think_and_ai() {
        let mut s = TuiState::new();
        s.current_think_text = "reasoning".to_string();
        s.current_ai_text = "response".to_string();
        s.flush_turn();
        assert!(s.current_think_text.is_empty());
        assert!(s.current_ai_text.is_empty());
        assert_eq!(s.messages.len(), 2);
        assert!(matches!(s.messages[0], ChatItem::Think(_)));
        assert!(matches!(s.messages[1], ChatItem::Ai(_)));
    }

    #[test]
    fn scroll_up_disables_auto_scroll() {
        let mut s = TuiState::new();
        s.auto_scroll = true;
        s.scroll_up(3);
        assert!(!s.auto_scroll);
        assert_eq!(s.scroll, 3);
    }

    #[test]
    fn scroll_down_to_zero_enables_auto_scroll() {
        let mut s = TuiState::new();
        s.auto_scroll = false;
        s.scroll = 1;
        s.scroll_down(1);
        assert_eq!(s.scroll, 0);
        assert!(s.auto_scroll);
    }

    #[test]
    fn push_user_adds_chat_item() {
        let mut s = TuiState::new();
        s.push_user("hi".to_string(), Some("10:00".to_string()));
        assert_eq!(s.messages.len(), 1);
        assert!(matches!(&s.messages[0], ChatItem::User(u) if u.text == "hi" && u.timestamp.as_deref() == Some("10:00")));
    }

    #[test]
    fn push_error_adds_error_item() {
        let mut s = TuiState::new();
        s.push_error("something failed".to_string(), None);
        assert_eq!(s.messages.len(), 1);
        assert!(matches!(&s.messages[0], ChatItem::Error(e) if e.text == "something failed"));
    }

    #[test]
    fn push_tool_adds_tool_item() {
        let mut s = TuiState::new();
        s.push_tool(ToolCallMessage::running("t1", "bash", Some("ls".into())));
        assert_eq!(s.messages.len(), 1);
        assert!(matches!(&s.messages[0], ChatItem::Tool(_)));
    }

    #[test]
    fn input_cursor_left_right() {
        let mut s = TuiState::new();
        s.input_insert('a');
        s.input_insert('b');
        s.input_insert('c');
        assert_eq!(s.input_cursor, 3);
        s.input_cursor_left();
        assert_eq!(s.input_cursor, 2);
        s.input_cursor_left();
        assert_eq!(s.input_cursor, 1);
        s.input_cursor_right();
        assert_eq!(s.input_cursor, 2);
    }

    #[test]
    fn input_cursor_left_at_zero() {
        let mut s = TuiState::new();
        s.input_cursor_left();
        assert_eq!(s.input_cursor, 0);
    }

    #[test]
    fn input_cursor_right_at_end() {
        let mut s = TuiState::new();
        s.input_insert('x');
        s.input_cursor_right();
        assert_eq!(s.input_cursor, 1); // already at end, no change
    }

    #[test]
    fn input_cursor_home_end() {
        let mut s = TuiState::new();
        s.input_insert('a');
        s.input_insert('b');
        s.input_insert('c');
        s.input_cursor_home();
        assert_eq!(s.input_cursor, 0);
        s.input_cursor_end();
        assert_eq!(s.input_cursor, 3);
    }

    #[test]
    fn input_delete_forward() {
        let mut s = TuiState::new();
        s.input_buffer = "abc".to_string();
        s.input_cursor = 1; // cursor after 'a'
        s.input_delete();
        assert_eq!(s.input_buffer, "ac");
        assert_eq!(s.input_cursor, 1);
    }

    #[test]
    fn input_delete_at_end_no_op() {
        let mut s = TuiState::new();
        s.input_buffer = "x".to_string();
        s.input_cursor = 1;
        s.input_delete();
        assert_eq!(s.input_buffer, "x");
    }

    #[test]
    fn input_clear_line() {
        let mut s = TuiState::new();
        s.input_buffer = "hello".to_string();
        s.input_cursor = 3;
        s.input_clear_line();
        assert!(s.input_buffer.is_empty());
        assert_eq!(s.input_cursor, 0);
    }

    #[test]
    fn input_kill_to_end() {
        let mut s = TuiState::new();
        s.input_buffer = "hello world".to_string();
        s.input_cursor = 5;
        s.input_kill_to_end();
        assert_eq!(s.input_buffer, "hello");
        assert_eq!(s.input_cursor, 5);
    }

    #[test]
    fn input_cursor_multibyte() {
        let mut s = TuiState::new();
        s.input_insert('你');
        s.input_insert('好');
        // cursor at end of "你好"
        s.input_cursor_left();
        // should be between 你 and 好
        assert_eq!(s.input_cursor, "你".len());
        s.input_cursor_left();
        assert_eq!(s.input_cursor, 0);
        s.input_cursor_right();
        assert_eq!(s.input_cursor, "你".len());
    }

    #[test]
    fn input_delete_multibyte() {
        let mut s = TuiState::new();
        s.input_buffer = "你好".to_string();
        s.input_cursor = 0;
        s.input_delete();
        assert_eq!(s.input_buffer, "好");
    }

    #[test]
    fn push_separator_adds_item() {
        let mut s = TuiState::new();
        s.push_separator("test".to_string());
        assert_eq!(s.messages.len(), 1);
        assert!(matches!(&s.messages[0], ChatItem::Separator(l) if l == "test"));
    }

    #[test]
    fn auto_scroll_off_preserves_scroll() {
        let mut s = TuiState::new();
        s.auto_scroll = false;
        s.scroll = 10;
        s.push_user("hi".to_string(), None);
        assert_eq!(s.scroll, 10); // should NOT reset when auto_scroll is off
    }

    #[test]
    fn cache_dirty_on_push() {
        let mut s = TuiState::new();
        s.cache_dirty = false;
        s.push_ai("test".to_string(), None);
        assert!(s.cache_dirty);
    }

    #[test]
    fn needs_redraw_on_input() {
        let mut s = TuiState::new();
        s.needs_redraw = false;
        s.input_insert('x');
        assert!(s.needs_redraw);
    }

    #[test]
    fn trace_lines_capped() {
        let mut s = TuiState::new();
        for i in 0..2500 {
            s.push_trace_line(format!("line {}", i));
        }
        assert!(s.trace_lines.len() <= 2000);
    }

    #[test]
    fn push_tool_grouped_single_stays_tool() {
        let mut s = TuiState::new();
        s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
        assert!(matches!(&s.messages[0], ChatItem::Tool(_)));
    }

    #[test]
    fn push_tool_grouped_second_upgrades_to_group() {
        let mut s = TuiState::new();
        s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
        s.push_tool_grouped(ToolCallMessage::running("t2", "grep", None));
        assert!(matches!(&s.messages[0], ChatItem::ToolGroup(g) if g.len() == 2));
    }

    #[test]
    fn push_tool_grouped_third_appends_to_group() {
        let mut s = TuiState::new();
        s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
        s.push_tool_grouped(ToolCallMessage::running("t2", "grep", None));
        s.push_tool_grouped(ToolCallMessage::running("t3", "read", None));
        assert!(matches!(&s.messages[0], ChatItem::ToolGroup(g) if g.len() == 3));
    }

    #[test]
    fn update_tool_by_id_in_group() {
        use crate::messages::tool::ToolCallStatus;
        let mut s = TuiState::new();
        s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
        s.push_tool_grouped(ToolCallMessage::running("t2", "grep", None));
        let updated = s.update_tool_by_id("t1", 200, true);
        assert!(updated);
        if let ChatItem::ToolGroup(g) = &s.messages[0] {
            assert!(matches!(&g[0].status, ToolCallStatus::Done { success: true, .. }));
            assert!(matches!(&g[1].status, ToolCallStatus::Running));
        } else {
            panic!("expected ToolGroup");
        }
    }

    #[test]
    fn update_tool_by_id_single_tool() {
        use crate::messages::tool::ToolCallStatus;
        let mut s = TuiState::new();
        s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
        let updated = s.update_tool_by_id("t1", 100, true);
        assert!(updated);
        assert!(matches!(&s.messages[0], ChatItem::Tool(t) if matches!(t.status, ToolCallStatus::Done { .. })));
    }

    #[test]
    fn non_consecutive_tools_are_separate() {
        let mut s = TuiState::new();
        s.push_tool_grouped(ToolCallMessage::running("t1", "bash", None));
        s.push_ai("between".to_string(), None); // breaks the consecutive run
        s.push_tool_grouped(ToolCallMessage::running("t2", "grep", None));
        assert_eq!(s.messages.len(), 3);
        assert!(matches!(&s.messages[0], ChatItem::Tool(_)));
        assert!(matches!(&s.messages[2], ChatItem::Tool(_)));
    }
}
