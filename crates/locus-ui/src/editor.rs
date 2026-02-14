//! External editor integration (from services).
//!
//! Open files in vim/nvim/nano with proper terminal state management.

use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Terminal;
use std::io::stdout;
use std::process::Command;

/// Detect available editor (preferred, then vim, nvim, nano).
pub fn detect_editor(preferred: Option<String>) -> Option<String> {
    if let Some(editor) = preferred {
        if is_editor_available(&editor) {
            return Some(editor);
        }
    }
    for editor in &["vim", "nvim", "nano"] {
        if is_editor_available(editor) {
            return Some(editor.to_string());
        }
    }
    None
}

fn is_editor_available(editor: &str) -> bool {
    Command::new("which")
        .arg(editor)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Suspend TUI, run editor, then restore TUI.
pub fn open_in_editor<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    editor: &str,
    file_path: &str,
    line_number: Option<usize>,
) -> Result<(), String> {
    execute!(stdout(), LeaveAlternateScreen, Show)
        .map_err(|e| format!("Failed to leave alternate screen: {}", e))?;
    disable_raw_mode().map_err(|e| format!("Failed to disable raw mode: {}", e))?;

    let mut cmd = Command::new(editor);
    cmd.arg(file_path);
    if let Some(line) = line_number {
        cmd.arg(format!("+{}", line));
    }
    let result = cmd.status();

    let restore_result = restore_tui(terminal);
    match result {
        Ok(status) if status.success() => restore_result,
        Ok(status) => Err(format!("Editor exited with code: {:?}", status.code())),
        Err(e) => {
            let _ = restore_result;
            Err(format!("Failed to run editor: {}", e))
        }
    }
}

fn restore_tui<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> Result<(), String> {
    execute!(stdout(), EnterAlternateScreen, Hide)
        .map_err(|e| format!("Failed to enter alternate screen: {}", e))?;
    enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {}", e))?;
    terminal
        .clear()
        .map_err(|e| format!("Failed to clear terminal: {}", e))?;
    Ok(())
}
