mod args;
mod error;

pub use args::CreateFileArgs;
pub use error::CreateFileError;

use crate::tools::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};

pub struct CreateFile {
    workspace_root: PathBuf,
}

impl CreateFile {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    fn validate_path(&self, path: &str) -> Result<PathBuf, CreateFileError> {
        let path = Path::new(path);

        // Handle empty path
        if path.as_os_str().is_empty() {
            return Err(CreateFileError::InvalidPath(
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
            return Err(CreateFileError::PathOutsideWorkspace(
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

impl Default for CreateFile {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

#[async_trait]
impl Tool for CreateFile {
    fn name(&self) -> &'static str {
        "create_file"
    }

    fn description(&self) -> &'static str {
        "Create or overwrite a file in the workspace. Creates parent directories if needed."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to create or overwrite (relative to workspace root)"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                },
                "create_dirs": {
                    "type": "boolean",
                    "description": "Whether to create parent directories if they don't exist (default: true)",
                    "default": true
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let tool_args: CreateFileArgs = serde_json::from_value(args)?;

        let file_path = self.validate_path(&tool_args.path)?;

        // Create parent directories if needed
        if tool_args.create_dirs {
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| CreateFileError::CreateDirsFailed(e.to_string()))?;
            }
        }

        // Write the file
        tokio::fs::write(&file_path, &tool_args.content)
            .await
            .map_err(|e| CreateFileError::WriteFailed(e.to_string()))?;

        // Get file metadata for response
        let metadata = tokio::fs::metadata(&file_path).await.ok();
        let size = metadata.map(|m| m.len()).unwrap_or(0);

        Ok(serde_json::json!({
            "success": true,
            "path": tool_args.path,
            "absolute_path": file_path.to_string_lossy(),
            "bytes_written": tool_args.content.len(),
            "size": size
        }))
    }
}
