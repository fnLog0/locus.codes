mod args;
mod error;

pub use args::{EditFileArgs, EditOperation};
pub use error::EditFileError;

use crate::history::EditHistory;
use crate::tools::{parse_tool_schema, Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::OnceLock;
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

fn schema() -> &'static (&'static str, &'static str, JsonValue) {
    static SCHEMA: OnceLock<(&'static str, &'static str, JsonValue)> = OnceLock::new();
    SCHEMA.get_or_init(|| parse_tool_schema(include_str!("schema.json")))
}

#[async_trait]
impl Tool for EditFile {
    fn name(&self) -> &'static str {
        schema().0
    }

    fn description(&self) -> &'static str {
        schema().1
    }

    fn parameters_schema(&self) -> JsonValue {
        schema().2.clone()
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let tool_args: EditFileArgs = serde_json::from_value(args)?;

        let file_path = self.validate_path(&tool_args.path)?;

        // Check if multiedit mode
        if let Some(edits) = &tool_args.edits {
            return self.execute_multiedit(file_path, edits).await;
        }

        // Single edit mode (backward compatible)
        self.execute_single_edit(file_path, tool_args).await
    }
}

impl EditFile {
    async fn execute_single_edit(
        &self,
        file_path: PathBuf,
        tool_args: EditFileArgs,
    ) -> ToolResult {
        let new_string = tool_args.new_string.as_ref().ok_or_else(|| {
            EditFileError::InvalidArgs("new_string is required when not using edits array".to_string())
        })?;

        // Check if this is an overwrite (empty or no old_string)
        let is_overwrite = tool_args.old_string.as_ref().map_or(true, |s| s.is_empty());

        if is_overwrite {
            // Overwrite mode: write new_string as full file content
            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| EditFileError::CreateDirsFailed(e.to_string()))?;
            }

            // Read existing content for history (if file exists)
            let old_content = tokio::fs::read_to_string(&file_path).await.unwrap_or_default();

            // Write the file
            tokio::fs::write(&file_path, new_string)
                .await
                .map_err(|e| EditFileError::WriteFailed(e.to_string()))?;

            // Record to history for undo
            let _ = self
                .history
                .record(&file_path, &old_content, &new_string)
                .await;

            let mut out = serde_json::json!({
                "success": true,
                "path": tool_args.path,
                "absolute_path": file_path.to_string_lossy(),
                "mode": "overwrite",
                "bytes_written": new_string.len()
            });
            if old_content.len() + new_string.len() <= MAX_DIFF_CONTENT_LEN {
                out["old_content"] = serde_json::Value::String(old_content);
                out["new_content"] = serde_json::Value::String(new_string.clone());
            }
            return Ok(out);
        }

        // Edit mode: find and replace
        self.execute_find_replace(file_path, tool_args).await
    }

    async fn execute_find_replace(
        &self,
        file_path: PathBuf,
        tool_args: EditFileArgs,
    ) -> ToolResult {
        // Edit mode: find and replace
        // Check if file exists
        if !tokio::fs::try_exists(&file_path).await.unwrap_or(false) {
            return Err(EditFileError::FileNotFound(tool_args.path).into());
        }

        // Read the file
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| EditFileError::ReadFailed(e.to_string()))?;

        let old_string = tool_args.old_string.as_ref().unwrap();
        let new_string = tool_args.new_string.as_ref().unwrap();

        // Count occurrences
        let match_count = content.matches(old_string).count();

        if match_count == 0 {
            return Err(EditFileError::OldStringNotFound.into());
        }

        // Check for multiple matches when replace_all is false
        if match_count > 1 && !tool_args.replace_all {
            return Err(EditFileError::MultipleMatches.into());
        }

        // Perform the replacement
        let new_content = if tool_args.replace_all {
            content.replace(old_string, new_string)
        } else {
            // Only replace first occurrence
            content.replacen(old_string, new_string, 1)
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

        let mut out = serde_json::json!({
            "success": true,
            "path": tool_args.path,
            "absolute_path": file_path.to_string_lossy(),
            "mode": "edit",
            "matches_found": match_count,
            "matches_replaced": if tool_args.replace_all { match_count } else { 1 }
        });
        if content.len() + new_content.len() <= MAX_DIFF_CONTENT_LEN {
            out["old_content"] = serde_json::Value::String(content);
            out["new_content"] = serde_json::Value::String(new_content);
        }
        Ok(out)
    }

    async fn execute_multiedit(
        &self,
        file_path: PathBuf,
        edits: &[EditOperation],
    ) -> ToolResult {

        // Check if file exists
        if !tokio::fs::try_exists(&file_path).await.unwrap_or(false) {
            return Err(EditFileError::FileNotFound(
                file_path.to_string_lossy().to_string(),
            )
            .into());
        }

        // Read the file
        let original_content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| EditFileError::ReadFailed(e.to_string()))?;

        let mut content = original_content.clone();
        let mut total_matches = 0;
        let mut total_replaced = 0;

        // Apply each edit in sequence
        for (index, edit) in edits.iter().enumerate() {
            let edit_num = index + 1;

            // Check if old_string is empty (overwrite in multiedit is not allowed)
            if edit.old_string.is_empty() {
                return Err(EditFileError::InvalidArgs(format!(
                    "Edit {}: old_string cannot be empty in multiedit mode",
                    edit_num
                ))
                .into());
            }

            // Count occurrences
            let match_count = content.matches(&edit.old_string).count();

            if match_count == 0 {
                return Err(EditFileError::MultieditStringNotFound { edit_number: edit_num }.into());
            }

            // Check for multiple matches when replace_all is false
            if match_count > 1 && !edit.replace_all {
                return Err(EditFileError::MultieditMultipleMatches { edit_number: edit_num }.into());
            }

            // Perform the replacement
            content = if edit.replace_all {
                total_matches += match_count;
                total_replaced += match_count;
                content.replace(&edit.old_string, &edit.new_string)
            } else {
                total_matches += 1;
                total_replaced += 1;
                content.replacen(&edit.old_string, &edit.new_string, 1)
            };
        }

        // Write the file
        tokio::fs::write(&file_path, &content)
            .await
            .map_err(|e| EditFileError::WriteFailed(e.to_string()))?;

        // Record to history for undo
        let _ = self
            .history
            .record(&file_path, &original_content, &content)
            .await;

        let mut out = serde_json::json!({
            "success": true,
            "path": file_path.to_string_lossy(),
            "absolute_path": file_path.to_string_lossy(),
            "mode": "multiedit",
            "edits_applied": edits.len(),
            "total_matches_found": total_matches,
            "total_matches_replaced": total_replaced
        });
        if original_content.len() + content.len() <= MAX_DIFF_CONTENT_LEN {
            out["old_content"] = serde_json::Value::String(original_content);
            out["new_content"] = serde_json::Value::String(content);
        }
        Ok(out)
    }
}

/// Max total old+new content size to include in result for TUI diff (bytes).
const MAX_DIFF_CONTENT_LEN: usize = 4096;
