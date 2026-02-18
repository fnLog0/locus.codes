mod args;
mod error;

pub use args::EditFileArgs;
pub use error::EditFileError;

use crate::history::EditHistory;
use crate::tools::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct EditFile {
    workspace_root: PathBuf,
    history: Arc<EditHistory>,
}

impl EditFile {
    pub fn new(workspace_root: PathBuf, history: Arc<EditHistory>) -> Self {
        Self {
            workspace_root,
            history,
        }
    }

    fn validate_path(&self, path: &str) -> Result<PathBuf, EditFileError> {
        let path = Path::new(path);

        // Handle empty path
        if path.as_os_str().is_empty() {
            return Err(EditFileError::InvalidPath(
                "Path cannot be empty".to_string(),
            ));
        }

        // Resolve the full path
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        // Normalize the path to remove .. and .
        let normalized_path = normalize_path(&full_path);

        // For security, check if the path would escape the workspace
        // We compare the normalized absolute path
        let normalized_workspace = normalize_path(&self.workspace_root);

        // Check if the normalized path starts with the normalized workspace
        if !is_within_directory(&normalized_path, &normalized_workspace) {
            return Err(EditFileError::PathOutsideWorkspace(
                full_path.to_string_lossy().to_string(),
            ));
        }

        Ok(normalized_path)
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn is_within_directory(path: &Path, dir: &Path) -> bool {
    path.starts_with(dir)
}

impl Default for EditFile {
    fn default() -> Self {
        Self::new(
            PathBuf::from("."),
            Arc::new(EditHistory::load_blocking(PathBuf::from("."))),
        )
    }
}

#[async_trait]
impl Tool for EditFile {
    fn name(&self) -> &'static str {
        "edit_file"
    }

    fn description(&self) -> &'static str {
        "Make edits to a text file by finding and replacing text. Supports single or all occurrences."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to edit (relative to workspace root)"
                },
                "old_string": {
                    "type": "string",
                    "description": "The text to find and replace (must match exactly including whitespace)"
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement text"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: false, only replace first)",
                    "default": false
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let tool_args: EditFileArgs = serde_json::from_value(args)?;

        let file_path = self.validate_path(&tool_args.path)?;

        // Check if file exists
        if !tokio::fs::try_exists(&file_path).await.unwrap_or(false) {
            return Err(EditFileError::FileNotFound(tool_args.path).into());
        }

        // Read the file
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| EditFileError::ReadFailed(e.to_string()))?;

        // Count occurrences
        let matches: Vec<usize> = content
            .match_indices(&tool_args.old_string)
            .map(|(i, _)| i)
            .collect();

        let match_count = matches.len();

        if match_count == 0 {
            return Err(EditFileError::OldStringNotFound.into());
        }

        // Check for multiple matches when replace_all is false
        if match_count > 1 && !tool_args.replace_all {
            return Err(EditFileError::MultipleMatches.into());
        }

        // Perform the replacement
        let new_content = if tool_args.replace_all {
            content.replace(&tool_args.old_string, &tool_args.new_string)
        } else {
            // Only replace first occurrence
            content.replacen(&tool_args.old_string, &tool_args.new_string, 1)
        };

        // Write the file
        tokio::fs::write(&file_path, &new_content)
            .await
            .map_err(|e| EditFileError::WriteFailed(e.to_string()))?;

        // Record to history for undo
        let _ = self
            .history
            .record(&file_path, &content, &new_content)
            .await;

        Ok(serde_json::json!({
            "success": true,
            "path": tool_args.path,
            "absolute_path": file_path.to_string_lossy(),
            "matches_found": match_count,
            "matches_replaced": if tool_args.replace_all { match_count } else { 1 }
        }))
    }
}
