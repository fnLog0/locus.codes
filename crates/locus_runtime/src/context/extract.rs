//! File path extraction from session turns.

use std::path::Path;

use locus_core::{ContentBlock, Session};

/// Extract recently mentioned files from session turns.
pub(crate) fn extract_recent_files(session: &Session) -> Vec<String> {
    let mut files = Vec::new();
    let file_keywords = ["file_path", "path", "file:"];

    for turn in session.turns.iter().rev().take(5) {
        for block in &turn.blocks {
            if let ContentBlock::Text { text } = block {
                for line in text.lines() {
                    for keyword in &file_keywords {
                        if line.contains(keyword) {
                            if let Some(path) = extract_path_from_line(line) {
                                if !files.contains(&path) {
                                    files.push(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    files.truncate(5);
    files
}

/// Try to extract a file path from a line of text.
fn extract_path_from_line(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for part in parts {
        if part.contains('/') && !part.starts_with(|c: char| c.is_ascii_punctuation()) {
            let cleaned = part.trim_matches(|c| c == '"' || c == '\'' || c == ',');
            if Path::new(cleaned).extension().is_some() || cleaned.contains('/') {
                return Some(cleaned.to_string());
            }
        }
    }
    None
}
