# Glow Rust - Key Bindings

This document covers keyboard handling and key bindings.

## Key Binding System

### Key Definition

```rust
// src/input/keys.rs

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A key binding that can match key events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Single character key
    Char(char),
    /// Character with Ctrl
    Ctrl(char),
    /// Character with Alt
    Alt(char),
    /// Character with Shift
    Shift(char),
    /// Enter key
    Enter,
    /// Escape key
    Esc,
    /// Tab key
    Tab,
    /// Backtab (Shift+Tab)
    BackTab,
    /// Backspace key
    Backspace,
    /// Delete key
    Delete,
    /// Arrow keys
    Up,
    Down,
    Left,
    Right,
    /// Navigation keys
    Home,
    End,
    PageUp,
    PageDown,
    /// Function keys
    F(u8),
}

impl Key {
    /// Check if this key matches a key event
    pub fn matches(&self, event: &KeyEvent) -> bool {
        match self {
            Key::Char(c) => {
                event.modifiers.is_empty() && event.code == KeyCode::Char(*c)
            }
            Key::Ctrl(c) => {
                event.modifiers.contains(KeyModifiers::CONTROL)
                    && !event.modifiers.contains(KeyModifiers::ALT)
                    && event.code == KeyCode::Char(*c)
            }
            Key::Alt(c) => {
                event.modifiers.contains(KeyModifiers::ALT)
                    && !event.modifiers.contains(KeyModifiers::CONTROL)
                    && event.code == KeyCode::Char(*c.to_ascii_lowercase())
            }
            Key::Shift(c) => {
                event.modifiers.contains(KeyModifiers::SHIFT)
                    && !event.modifiers.contains(KeyModifiers::CONTROL)
                    && !event.modifiers.contains(KeyModifiers::ALT)
                    && event.code == KeyCode::Char(*c)
            }
            Key::Enter => event.code == KeyCode::Enter,
            Key::Esc => event.code == KeyCode::Esc,
            Key::Tab => {
                event.code == KeyCode::Tab
                    && !event.modifiers.contains(KeyModifiers::SHIFT)
            }
            Key::BackTab => {
                event.code == KeyCode::BackTab
                    || (event.code == KeyCode::Tab
                        && event.modifiers.contains(KeyModifiers::SHIFT))
            }
            Key::Backspace => event.code == KeyCode::Backspace,
            Key::Delete => event.code == KeyCode::Delete,
            Key::Up => event.code == KeyCode::Up,
            Key::Down => event.code == KeyCode::Down,
            Key::Left => event.code == KeyCode::Left,
            Key::Right => event.code == KeyCode::Right,
            Key::Home => event.code == KeyCode::Home,
            Key::End => event.code == KeyCode::End,
            Key::PageUp => event.code == KeyCode::PageUp,
            Key::PageDown => event.code == KeyCode::PageDown,
            Key::F(n) => event.code == KeyCode::F(*n),
        }
    }
    
    /// Get a display string for this key
    pub fn display(&self) -> String {
        match self {
            Key::Char(c) => c.to_string(),
            Key::Ctrl(c) => format!("Ctrl+{}", c),
            Key::Alt(c) => format!("Alt+{}", c),
            Key::Shift(c) => format!("Shift+{}", c),
            Key::Enter => "Enter".to_string(),
            Key::Esc => "Esc".to_string(),
            Key::Tab => "Tab".to_string(),
            Key::BackTab => "Shift+Tab".to_string(),
            Key::Backspace => "Backspace".to_string(),
            Key::Delete => "Delete".to_string(),
            Key::Up => "↑".to_string(),
            Key::Down => "↓".to_string(),
            Key::Left => "←".to_string(),
            Key::Right => "→".to_string(),
            Key::Home => "Home".to_string(),
            Key::End => "End".to_string(),
            Key::PageUp => "PgUp".to_string(),
            Key::PageDown => "PgDn".to_string(),
            Key::F(n) => format!("F{}", n),
        }
    }
}

/// Check if any of the keys match
pub fn matches_any(event: &KeyEvent, keys: &[Key]) -> bool {
    keys.iter().any(|k| k.matches(event))
}
```

### Key Bindings Configuration

```rust
// src/input/bindings.rs

use super::keys::Key;

/// Global key bindings
pub struct GlobalKeyBindings {
    pub quit: Vec<Key>,
    pub suspend: Vec<Key>,
    pub cancel: Vec<Key>,
}

impl GlobalKeyBindings {
    pub fn new() -> Self {
        Self {
            quit: vec![Key::Char('q'), Key::Ctrl('c')],
            suspend: vec![Key::Ctrl('z')],
            cancel: vec![Key::Esc],
        }
    }
    
    /// Check if key is quit
    pub fn is_quit(&self, event: &KeyEvent) -> bool {
        matches_any(event, &self.quit)
    }
    
    /// Check if key is suspend
    pub fn is_suspend(&self, event: &KeyEvent) -> bool {
        matches_any(event, &self.suspend)
    }
}

/// Navigation key bindings
pub struct NavigationBindings {
    pub up: Vec<Key>,
    pub down: Vec<Key>,
    pub left: Vec<Key>,
    pub right: Vec<Key>,
    pub page_up: Vec<Key>,
    pub page_down: Vec<Key>,
    pub half_up: Vec<Key>,
    pub half_down: Vec<Key>,
    pub home: Vec<Key>,
    pub end: Vec<Key>,
}

impl NavigationBindings {
    pub fn new() -> Self {
        Self {
            up: vec![Key::Char('k'), Key::Up],
            down: vec![Key::Char('j'), Key::Down],
            left: vec![Key::Char('h'), Key::Left],
            right: vec![Key::Char('l'), Key::Right],
            page_up: vec![Key::Char('b'), Key::PageUp],
            page_down: vec![Key::Char('f'), Key::PageDown],
            half_up: vec![Key::Char('u')],
            half_down: vec![Key::Char('d')],
            home: vec![Key::Char('g'), Key::Home],
            end: vec![Key::Shift('G'), Key::End],
        }
    }
}

/// Stash (file listing) key bindings
pub struct StashBindings {
    pub open: Vec<Key>,
    pub edit: Vec<Key>,
    pub filter: Vec<Key>,
    pub clear_filter: Vec<Key>,
    pub refresh: Vec<Key>,
    pub help: Vec<Key>,
    pub next_section: Vec<Key>,
    pub prev_section: Vec<Key>,
    pub next_page: Vec<Key>,
    pub prev_page: Vec<Key>,
    pub show_errors: Vec<Key>,
}

impl StashBindings {
    pub fn new() -> Self {
        Self {
            open: vec![Key::Enter],
            edit: vec![Key::Char('e')],
            filter: vec![Key::Char('/')],
            clear_filter: vec![Key::Esc],
            refresh: vec![Key::Char('r'), Key::Char('F')],
            help: vec![Key::Char('?')],
            next_section: vec![Key::Tab, Key::Shift('L')],
            prev_section: vec![Key::BackTab, Key::Shift('H')],
            next_page: vec![Key::Right, Key::Char('l'), Key::Char('f'), Key::Char('d')],
            prev_page: vec![Key::Left, Key::Char('h'), Key::Char('b'), Key::Char('u')],
            show_errors: vec![Key::Shift('!')],
        }
    }
}

/// Pager (document view) key bindings  
pub struct PagerBindings {
    pub back: Vec<Key>,
    pub edit: Vec<Key>,
    pub copy: Vec<Key>,
    pub reload: Vec<Key>,
    pub help: Vec<Key>,
    pub toggle_line_numbers: Vec<Key>,
}

impl PagerBindings {
    pub fn new() -> Self {
        Self {
            back: vec![Key::Esc, Key::Char('q'), Key::Char('h'), Key::Left],
            edit: vec![Key::Char('e')],
            copy: vec![Key::Char('c')],
            reload: vec![Key::Char('r')],
            help: vec![Key::Char('?')],
            toggle_line_numbers: vec![Key::Char('n')],
        }
    }
}

/// Filter mode key bindings
pub struct FilterBindings {
    pub confirm: Vec<Key>,
    pub cancel: Vec<Key>,
    pub up: Vec<Key>,
    pub down: Vec<Key>,
}

impl FilterBindings {
    pub fn new() -> Self {
        Self {
            confirm: vec![Key::Enter, Key::Tab],
            cancel: vec![Key::Esc],
            up: vec![Key::Ctrl('k'), Key::Up],
            down: vec![Key::Ctrl('j'), Key::Down],
        }
    }
}

/// All key bindings combined
pub struct KeyBindings {
    pub global: GlobalKeyBindings,
    pub navigation: NavigationBindings,
    pub stash: StashBindings,
    pub pager: PagerBindings,
    pub filter: FilterBindings,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            global: GlobalKeyBindings::new(),
            navigation: NavigationBindings::new(),
            stash: StashBindings::new(),
            pager: PagerBindings::new(),
            filter: FilterBindings::new(),
        }
    }
}

impl KeyBindings {
    pub fn new() -> Self {
        Self::default()
    }
}
```

## Key Handler

```rust
// src/input/handler.rs

use crossterm::event::{KeyEvent, KeyEventKind};
use super::bindings::KeyBindings;

/// Result of handling a key
#[derive(Debug)]
pub enum KeyResult {
    /// Key was handled, no further action needed
    Handled,
    /// Key was not handled
    Unhandled,
    /// Request to quit the application
    Quit,
    /// Request to suspend the application
    Suspend,
    /// Open the selected document
    OpenDocument,
    /// Edit the selected/current document
    EditDocument { line: usize },
    /// Copy current document content
    CopyContent,
    /// Reload current document
    ReloadDocument,
    /// Go back to stash from pager
    BackToStash,
    /// Start filtering
    StartFilter,
    /// Clear filter
    ClearFilter,
    /// Confirm filter selection
    ConfirmFilter,
    /// Refresh file list
    RefreshFiles,
    /// Toggle help
    ToggleHelp,
    /// Navigate sections
    NextSection,
    PrevSection,
    /// Navigate pages
    NextPage,
    PrevPage,
    /// Show errors
    ShowErrors,
}

/// Key event handler
pub struct KeyHandler {
    bindings: KeyBindings,
}

impl KeyHandler {
    pub fn new() -> Self {
        Self {
            bindings: KeyBindings::default(),
        }
    }
    
    pub fn with_bindings(bindings: KeyBindings) -> Self {
        Self { bindings }
    }
    
    /// Handle a global key (works in any context)
    pub fn handle_global(&self, event: &KeyEvent) -> Option<KeyResult> {
        if self.bindings.global.is_quit(event) {
            return Some(KeyResult::Quit);
        }
        
        if self.bindings.global.is_suspend(event) {
            return Some(KeyResult::Suspend);
        }
        
        None
    }
    
    /// Handle a key in the stash context
    pub fn handle_stash(&self, event: &KeyEvent, is_filtering: bool) -> Option<KeyResult> {
        // If filtering, use filter bindings
        if is_filtering {
            return self.handle_filter(event);
        }
        
        // Navigation
        if matches_any(event, &self.bindings.navigation.up) {
            return Some(KeyResult::Handled); // Caller handles cursor movement
        }
        if matches_any(event, &self.bindings.navigation.down) {
            return Some(KeyResult::Handled);
        }
        
        // Actions
        if matches_any(event, &self.bindings.stash.open) {
            return Some(KeyResult::OpenDocument);
        }
        if matches_any(event, &self.bindings.stash.edit) {
            return Some(KeyResult::EditDocument { line: 0 });
        }
        if matches_any(event, &self.bindings.stash.filter) {
            return Some(KeyResult::StartFilter);
        }
        if matches_any(event, &self.bindings.stash.clear_filter) {
            return Some(KeyResult::ClearFilter);
        }
        if matches_any(event, &self.bindings.stash.refresh) {
            return Some(KeyResult::RefreshFiles);
        }
        if matches_any(event, &self.bindings.stash.help) {
            return Some(KeyResult::ToggleHelp);
        }
        if matches_any(event, &self.bindings.stash.next_section) {
            return Some(KeyResult::NextSection);
        }
        if matches_any(event, &self.bindings.stash.prev_section) {
            return Some(KeyResult::PrevSection);
        }
        if matches_any(event, &self.bindings.stash.next_page) {
            return Some(KeyResult::NextPage);
        }
        if matches_any(event, &self.bindings.stash.prev_page) {
            return Some(KeyResult::PrevPage);
        }
        if matches_any(event, &self.bindings.stash.show_errors) {
            return Some(KeyResult::ShowErrors);
        }
        
        None
    }
    
    /// Handle a key in the pager context
    pub fn handle_pager(&self, event: &KeyEvent) -> Option<KeyResult> {
        // Navigation (caller handles actual scrolling)
        if matches_any(event, &self.bindings.navigation.up) {
            return Some(KeyResult::Handled);
        }
        if matches_any(event, &self.bindings.navigation.down) {
            return Some(KeyResult::Handled);
        }
        if matches_any(event, &self.bindings.navigation.home) {
            return Some(KeyResult::Handled);
        }
        if matches_any(event, &self.bindings.navigation.end) {
            return Some(KeyResult::Handled);
        }
        if matches_any(event, &self.bindings.navigation.half_up) {
            return Some(KeyResult::Handled);
        }
        if matches_any(event, &self.bindings.navigation.half_down) {
            return Some(KeyResult::Handled);
        }
        
        // Actions
        if matches_any(event, &self.bindings.pager.back) {
            return Some(KeyResult::BackToStash);
        }
        if matches_any(event, &self.bindings.pager.edit) {
            return Some(KeyResult::EditDocument { line: 0 }); // Will calculate actual line
        }
        if matches_any(event, &self.bindings.pager.copy) {
            return Some(KeyResult::CopyContent);
        }
        if matches_any(event, &self.bindings.pager.reload) {
            return Some(KeyResult::ReloadDocument);
        }
        if matches_any(event, &self.bindings.pager.help) {
            return Some(KeyResult::ToggleHelp);
        }
        
        None
    }
    
    /// Handle a key in filter mode
    pub fn handle_filter(&self, event: &KeyEvent) -> Option<KeyResult> {
        if matches_any(event, &self.bindings.filter.confirm) {
            return Some(KeyResult::ConfirmFilter);
        }
        if matches_any(event, &self.bindings.filter.cancel) {
            return Some(KeyResult::ClearFilter);
        }
        // Up/down for navigation in filter results
        if matches_any(event, &self.bindings.filter.up) {
            return Some(KeyResult::Handled);
        }
        if matches_any(event, &self.bindings.filter.down) {
            return Some(KeyResult::Handled);
        }
        
        None
    }
}
```

## Key Reference Tables

### Global Keys

| Key | Action | Notes |
|-----|--------|-------|
| `q` | Quit | Works everywhere |
| `Ctrl+C` | Quit | Force quit |
| `Ctrl+Z` | Suspend | Unix only, SIGTSTP |

### Stash (File Listing) Keys

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` / `Home` | Go to first item |
| `G` / `End` | Go to last item |
| `Enter` | Open document |
| `e` | Edit in $EDITOR |
| `/` | Start filter/search |
| `Esc` | Clear filter |
| `r` / `F` | Refresh file list |
| `?` | Toggle help |
| `Tab` / `L` | Next section |
| `Shift+Tab` / `H` | Previous section |
| `h` / `l` / `←` / `→` | Previous/next page |
| `b` / `f` | Previous/next page (vim-style) |
| `!` | Show errors |

### Pager (Document View) Keys

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `g` / `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `d` | Half page down |
| `u` | Half page up |
| `f` / `Space` / `PageDown` | Page down |
| `b` / `PageUp` | Page up |
| `Esc` / `q` / `h` / `←` | Back to file list |
| `e` | Edit at current line |
| `c` | Copy content to clipboard |
| `r` | Reload document |
| `?` | Toggle help |

### Filter Mode Keys

| Key | Action |
|-----|--------|
| `Enter` / `Tab` | Confirm selection |
| `Esc` | Cancel filter |
| `Ctrl+J` / `↓` | Next result |
| `Ctrl+K` / `↑` | Previous result |
| Any character | Add to filter |

## Help View

```rust
// src/ui/help.rs

use ratatui::{
    text::{Line, Span},
    style::{Color, Style},
};

/// Help entry for display
pub struct HelpEntry {
    pub keys: Vec<String>,
    pub description: String,
}

impl HelpEntry {
    pub fn new(keys: &[&str], description: &str) -> Self {
        Self {
            keys: keys.iter().map(|s| s.to_string()).collect(),
            description: description.to_string(),
        }
    }
    
    /// Render as a styled line
    pub fn render(&self, key_style: Style, desc_style: Style) -> Line {
        let keys_str = self.keys.join(" / ");
        Line::from(vec![
            Span::styled(keys_str, key_style),
            Span::raw("  "),
            Span::styled(&self.description, desc_style),
        ])
    }
}

/// Stash help entries
pub fn stash_help() -> Vec<HelpEntry> {
    vec![
        HelpEntry::new(&["j/k", "↑/↓"], "navigate"),
        HelpEntry::new(&["g", "Home"], "go to start"),
        HelpEntry::new(&["G", "End"], "go to end"),
        HelpEntry::new(&["Enter"], "open"),
        HelpEntry::new(&["e"], "edit"),
        HelpEntry::new(&["/"], "find"),
        HelpEntry::new(&["r"], "refresh"),
        HelpEntry::new(&["Tab"], "section"),
        HelpEntry::new(&["h/l", "←/→"], "page"),
        HelpEntry::new(&["?"], "help"),
        HelpEntry::new(&["q"], "quit"),
    ]
}

/// Pager help entries
pub fn pager_help() -> Vec<HelpEntry> {
    vec![
        HelpEntry::new(&["j/k", "↑/↓"], "scroll"),
        HelpEntry::new(&["g", "Home"], "go to top"),
        HelpEntry::new(&["G", "End"], "go to bottom"),
        HelpEntry::new(&["d"], "half page down"),
        HelpEntry::new(&["u"], "half page up"),
        HelpEntry::new(&["c"], "copy contents"),
        HelpEntry::new(&["e"], "edit"),
        HelpEntry::new(&["r"], "reload"),
        HelpEntry::new(&["Esc"], "back to files"),
        HelpEntry::new(&["q"], "quit"),
    ]
}

/// Render help as multi-column layout
pub fn render_help_columns(entries: &[HelpEntry], width: u16, key_style: Style, desc_style: Style) -> ratatui::text::Text {
    let mut lines = Vec::new();
    
    // Calculate column width
    let col_width = (width as usize) / 2;
    
    // Group into two columns
    let mid = (entries.len() + 1) / 2;
    
    for i in 0..mid {
        let mut spans = Vec::new();
        
        // Left column
        if let Some(entry) = entries.get(i) {
            let line = entry.render(key_style, desc_style);
            let padding = col_width.saturating_sub(line.width());
            spans.extend(line.spans);
            spans.push(Span::raw(" ".repeat(padding)));
        }
        
        // Right column
        if let Some(entry) = entries.get(i + mid) {
            let line = entry.render(key_style, desc_style);
            spans.extend(line.spans);
        }
        
        lines.push(Line::from(spans));
    }
    
    ratatui::text::Text::from(lines)
}
```
