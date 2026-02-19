//! Application state for the terminal UI.
//!
//! Manages all UI state including messages, input, layout, and visibility flags.

use std::path::PathBuf;

use crate::components::{Chat, Header, Input, ShortcutsBar};
use crate::theme::Theme;

/// Main application state for the TUI.
#[derive(Debug)]
pub struct AppState {
    // Core components
    pub chat: Chat,
    pub header: Header,
    pub input: Input,
    pub shortcuts: ShortcutsBar,
    pub theme: Theme,

    // Layout state
    pub show_side_panel: bool,
    pub side_panel_width: u16,

    // Mode flags
    pub loading: bool,
    pub shell_mode: bool,

    // Repository context
    pub repo_root: Option<PathBuf>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create a new app state with default values.
    pub fn new() -> Self {
        Self {
            chat: Chat::new(),
            header: Header::new(),
            input: Input::new(),
            shortcuts: ShortcutsBar::new(),
            theme: Theme::default(),

            show_side_panel: false,
            side_panel_width: 32,

            loading: false,
            shell_mode: false,

            repo_root: None,
        }
    }

    /// Set the repository root directory.
    pub fn set_repo_root(&mut self, path: PathBuf) {
        self.repo_root = Some(path.clone());
        self.header.update_directory(path);
    }

    /// Set the git branch.
    pub fn set_branch(&mut self, branch: Option<String>) {
        self.header.update_branch(branch);
    }

    /// Toggle the side panel visibility.
    pub fn toggle_side_panel(&mut self) {
        self.show_side_panel = !self.show_side_panel;
    }

    /// Toggle shell mode.
    pub fn toggle_shell_mode(&mut self) {
        self.shell_mode = !self.shell_mode;
    }

    /// Toggle dark/light theme.
    pub fn toggle_theme(&mut self) {
        self.theme.toggle();
    }

    /// Update the header time.
    pub fn update_time(&mut self) {
        self.header.update_time();
    }
}
