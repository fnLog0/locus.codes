mod args;
mod error;

pub use args::{GlobArgs, GlobResult};
pub use error::GlobError;

use crate::tools::{parse_tool_schema, Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::OnceLock;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

pub struct Glob {
    repo_root: Arc<std::path::PathBuf>,
}

impl Glob {
    pub fn new(repo_root: std::path::PathBuf) -> Self {
        Self {
            repo_root: Arc::new(repo_root),
        }
    }

    fn should_include(&self, path: &Path, args: &GlobArgs) -> bool {
        let path_str = path.to_string_lossy();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        for exclude in &args.exclude {
            if path_str.contains(exclude) || file_name == exclude {
                return false;
            }
        }

        true
    }

    async fn walk_directory(
        &self,
        dir: &Path,
        pattern: &glob::Pattern,
        args: &GlobArgs,
        result: &mut GlobResult,
    ) -> Result<(), GlobError> {
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if result.files.len() >= args.max_results {
                result.truncated = true;
                return Ok(());
            }

            let rel_path = path
                .strip_prefix(&*self.repo_root)
                .unwrap_or(&path)
                .to_string_lossy();

            if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if dir_name.starts_with('.')
                    || dir_name == "target"
                    || dir_name == "node_modules"
                    || dir_name == "vendor"
                    || args.exclude.iter().any(|e| dir_name == e)
                {
                    continue;
                }

                if pattern.matches(&rel_path) && args.include_dirs {
                    result.add_file(rel_path.to_string());
                }

                Box::pin(self.walk_directory(&path, pattern, args, result)).await?;
            } else if path.is_file()
                && self.should_include(&path, args)
                && pattern.matches(&rel_path)
            {
                result.add_file(rel_path.to_string());
            }
        }

        Ok(())
    }
}

fn schema() -> &'static (&'static str, &'static str, JsonValue) {
    static SCHEMA: OnceLock<(&'static str, &'static str, JsonValue)> = OnceLock::new();
    SCHEMA.get_or_init(|| parse_tool_schema(include_str!("schema.json")))
}

#[async_trait]
impl Tool for Glob {
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
        let glob_args: GlobArgs = serde_json::from_value(args)?;

        let pattern = glob::Pattern::new(&glob_args.pattern)
            .map_err(|e| anyhow::anyhow!("Invalid glob pattern: {}", e))?;

        let search_path = if let Some(ref path) = glob_args.path {
            let full_path = self.repo_root.join(path);
            if !full_path.exists() {
                return Err(anyhow::anyhow!("Path does not exist: {}", path));
            }
            full_path
        } else {
            self.repo_root.as_path().to_path_buf()
        };

        let mut result = GlobResult::new(glob_args.pattern.clone());

        if search_path.is_file() {
            let rel_path = search_path
                .strip_prefix(&*self.repo_root)
                .unwrap_or(&search_path)
                .to_string_lossy();
            if pattern.matches(&rel_path) {
                result.add_file(rel_path.to_string());
            }
        } else {
            self.walk_directory(&search_path, &pattern, &glob_args, &mut result)
                .await?;
        }

        Ok(result.to_json())
    }
}
