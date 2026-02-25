mod args;
mod error;

pub use args::UndoEditArgs;
pub use error::UndoEditError;

use crate::history::EditHistory;
use crate::tools::{parse_tool_schema, Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::OnceLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct UndoEdit {
    workspace_root: PathBuf,
    history: Arc<EditHistory>,
}

impl UndoEdit {
    pub fn new(workspace_root: PathBuf, history: Arc<EditHistory>) -> Self {
        Self {
            workspace_root,
            history,
        }
    }

    fn validate_path(&self, path: &str) -> Result<PathBuf, UndoEditError> {
        let path = Path::new(path);

        if path.as_os_str().is_empty() {
            return Err(UndoEditError::InvalidPath(
                "Path cannot be empty".to_string(),
            ));
        }

        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        let normalized_path = normalize_path(&full_path);
        let normalized_workspace = normalize_path(&self.workspace_root);

        if !normalized_path.starts_with(&normalized_workspace) {
            return Err(UndoEditError::PathOutsideWorkspace(
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

fn schema() -> &'static (&'static str, &'static str, JsonValue) {
    static SCHEMA: OnceLock<(&'static str, &'static str, JsonValue)> = OnceLock::new();
    SCHEMA.get_or_init(|| parse_tool_schema(include_str!("schema.json")))
}

#[async_trait]
impl Tool for UndoEdit {
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
        let tool_args: UndoEditArgs = serde_json::from_value(args)?;

        let file_path = self.validate_path(&tool_args.path)?;

        if !tokio::fs::try_exists(&file_path).await.unwrap_or(false) {
            return Err(UndoEditError::FileNotFound(tool_args.path).into());
        }

        let restored = self.history.undo(&file_path).await?;

        match restored {
            Some(_) => Ok(serde_json::json!({
                "success": true,
                "path": tool_args.path,
                "message": "Reverted to previous content"
            })),
            None => Err(UndoEditError::NothingToUndo.into()),
        }
    }
}
