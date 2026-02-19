mod args;
mod error;

pub use args::ReadArgs;
pub use error::ReadError;

use crate::tools::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct Read {
    repo_root: PathBuf,
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

impl Read {
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    fn validate_path(&self, path: &str) -> Result<PathBuf, ReadError> {
        let path = Path::new(path);

        if path.as_os_str().is_empty() {
            return Err(ReadError::InvalidPath("Path cannot be empty".to_string()));
        }

        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.repo_root.join(path)
        };

        let normalized_path = normalize_path(&full_path);
        let normalized_workspace = normalize_path(&self.repo_root);

        if !is_within_directory(&normalized_path, &normalized_workspace) {
            return Err(ReadError::PathOutsideWorkspace(
                full_path.to_string_lossy().to_string(),
            ));
        }

        Ok(normalized_path)
    }
}

#[async_trait]
impl Tool for Read {
    fn name(&self) -> &'static str {
        "read"
    }

    fn description(&self) -> &'static str {
        "Read a file or list a directory from the file system. Returns file contents as text or directory entries with name and type."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file or directory (relative to repo root)"
                },
                "max_bytes": {
                    "type": "integer",
                    "description": "Maximum bytes to read from a file (default: 1048576). Ignored when listing a directory.",
                    "default": 1048576
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let tool_args: ReadArgs = serde_json::from_value(args)?;
        let full_path = self.validate_path(&tool_args.path)?;

        let metadata = fs::metadata(&full_path)
            .await
            .map_err(|e| ReadError::NotFound(format!("{}: {}", tool_args.path, e)))?;

        if metadata.is_file() {
            let content = fs::read(&full_path).await.map_err(|e| {
                ReadError::IoError(std::io::Error::new(e.kind(), format!("read file: {}", e)))
            })?;

            let size_bytes = content.len();
            let (text, truncated) = if content.len() as u64 > tool_args.max_bytes {
                let limited = &content[..tool_args.max_bytes as usize];
                let s = String::from_utf8(limited.to_vec()).map_err(|_| ReadError::NotUtf8)?;
                (s, true)
            } else {
                let s = String::from_utf8(content).map_err(|_| ReadError::NotUtf8)?;
                (s, false)
            };

            Ok(serde_json::json!({
                "type": "file",
                "path": tool_args.path,
                "content": text,
                "truncated": truncated,
                "size_bytes": size_bytes
            }))
        } else if metadata.is_dir() {
            let mut entries = Vec::new();
            let mut read_dir = fs::read_dir(&full_path).await.map_err(|e| {
                ReadError::IoError(std::io::Error::new(e.kind(), format!("read dir: {}", e)))
            })?;

            while let Some(entry) = read_dir.next_entry().await? {
                let name = entry.file_name().to_string_lossy().into_owned();
                let entry_type = if entry.metadata().await?.is_dir() {
                    "dir"
                } else {
                    "file"
                };
                entries.push(serde_json::json!({ "name": name, "type": entry_type }));
            }

            entries.sort_by(|a, b| {
                let a_name = a["name"].as_str().unwrap_or("");
                let b_name = b["name"].as_str().unwrap_or("");
                a_name.cmp(b_name)
            });

            Ok(serde_json::json!({
                "type": "directory",
                "path": tool_args.path,
                "entries": entries
            }))
        } else {
            let e = ReadError::NotFound(format!("{}: not a file or directory", tool_args.path));
            Err(e.into())
        }
    }
}
