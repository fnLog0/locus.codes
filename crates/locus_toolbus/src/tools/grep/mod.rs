mod args;
mod error;

pub use args::{GrepArgs, GrepMatch, GrepResult};
pub use error::GrepError;

use crate::tools::{Tool, ToolResult};
use async_trait::async_trait;
use regex::RegexBuilder;
use serde_json::Value as JsonValue;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

pub struct Grep {
    repo_root: Arc<std::path::PathBuf>,
}

impl Grep {
    pub fn new(repo_root: std::path::PathBuf) -> Self {
        Self {
            repo_root: Arc::new(repo_root),
        }
    }

    fn build_regex(&self, args: &GrepArgs) -> Result<regex::Regex, GrepError> {
        if args.pattern.is_empty() {
            return Err(GrepError::EmptyPattern);
        }

        let pattern = if args.regex {
            args.pattern.clone()
        } else {
            regex::escape(&args.pattern)
        };

        RegexBuilder::new(&pattern)
            .case_insensitive(!args.case_sensitive)
            .build()
            .map_err(GrepError::from)
    }

    async fn search_file(
        &self,
        path: &Path,
        regex: &regex::Regex,
        args: &GrepArgs,
        result: &mut GrepResult,
    ) -> Result<bool, GrepError> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| GrepError::ReadError(e.to_string()))?;

        let lines: Vec<&str> = content.lines().collect();
        let rel_path = path
            .strip_prefix(&*self.repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let mut file_has_match = false;

        for (line_idx, line) in lines.iter().enumerate() {
            if result.matches.len() >= args.max_results {
                result.truncated = true;
                return Ok(true);
            }

            let line_number = line_idx + 1;

            for mat in regex.find_iter(line) {
                if result.matches.len() >= args.max_results {
                    result.truncated = true;
                    return Ok(true);
                }

                if !file_has_match {
                    file_has_match = true;
                    result.files_with_matches += 1;
                }

                if args.files_only {
                    continue;
                }

                let context_start = line_idx.saturating_sub(args.context_lines);
                let context_end = (line_idx + args.context_lines + 1).min(lines.len());

                let context_before: Vec<String> = lines[context_start..line_idx]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                let context_after: Vec<String> = lines[line_idx + 1..context_end]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                result.matches.push(GrepMatch {
                    file: rel_path.clone(),
                    line_number,
                    column: mat.start() + 1,
                    line: line.to_string(),
                    context_before,
                    context_after,
                    match_start: mat.start(),
                    match_end: mat.end(),
                });

                result.total_matches += 1;
            }
        }

        Ok(file_has_match)
    }

    async fn search_files(
        &self,
        files: &[std::path::PathBuf],
        regex: &regex::Regex,
        args: &GrepArgs,
        result: &mut GrepResult,
    ) -> Result<(), GrepError> {
        for file_path in files {
            if result.truncated {
                return Ok(());
            }

            if !file_path.is_file() {
                continue;
            }

            let file_has_match = self.search_file(file_path, regex, args, result).await?;

            if file_has_match && args.files_only {
                let rel_path = file_path
                    .strip_prefix(&*self.repo_root)
                    .unwrap_or(file_path)
                    .to_string_lossy()
                    .to_string();

                result.matches.push(GrepMatch {
                    file: rel_path,
                    line_number: 0,
                    column: 0,
                    line: String::new(),
                    context_before: Vec::new(),
                    context_after: Vec::new(),
                    match_start: 0,
                    match_end: 0,
                });
            }
        }

        Ok(())
    }

    pub async fn search_with_files(
        &self,
        files: &[std::path::PathBuf],
        args: GrepArgs,
    ) -> Result<GrepResult, GrepError> {
        let regex = self.build_regex(&args)?;
        let mut result = GrepResult::new(args.pattern.clone());
        self.search_files(files, &regex, &args, &mut result).await?;
        Ok(result)
    }

    async fn search_directory(
        &self,
        dir: &Path,
        regex: &regex::Regex,
        args: &GrepArgs,
        result: &mut GrepResult,
    ) -> Result<(), GrepError> {
        let mut entries = fs::read_dir(dir)
            .await
            .map_err(|e| GrepError::ReadError(e.to_string()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| GrepError::ReadError(e.to_string()))?
        {
            let path = entry.path();

            if result.truncated {
                return Ok(());
            }

            if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if dir_name.starts_with('.')
                    || dir_name == "target"
                    || dir_name == "node_modules"
                    || dir_name == "vendor"
                    || dir_name == ".git"
                {
                    continue;
                }

                Box::pin(self.search_directory(&path, regex, args, result)).await?;
            } else if path.is_file() {
                let file_has_match = self.search_file(&path, regex, args, result).await?;

                if file_has_match && args.files_only {
                    let rel_path = path
                        .strip_prefix(&*self.repo_root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();

                    result.matches.push(GrepMatch {
                        file: rel_path,
                        line_number: 0,
                        column: 0,
                        line: String::new(),
                        context_before: Vec::new(),
                        context_after: Vec::new(),
                        match_start: 0,
                        match_end: 0,
                    });
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Tool for Grep {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Fast text search tool. Searches for patterns in file contents using regex or literal matching. Use for finding text across the codebase."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The search pattern (literal text or regex)"
                },
                "path": {
                    "type": "string",
                    "description": "Relative path to search within (optional, defaults to repo root)"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Whether the search should be case sensitive (default: false)",
                    "default": false
                },
                "regex": {
                    "type": "boolean",
                    "description": "Treat pattern as a regex (default: false, treats as literal)",
                    "default": false
                },
                "context_lines": {
                    "type": "integer",
                    "description": "Number of context lines before and after match (default: 2)",
                    "default": 2
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 100)",
                    "default": 100
                },
                "files_only": {
                    "type": "boolean",
                    "description": "Only return file names with matches, not individual matches (default: false)",
                    "default": false
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let grep_args: GrepArgs = serde_json::from_value(args)?;

        let regex = self.build_regex(&grep_args)?;

        let search_path = if let Some(ref path) = grep_args.path {
            let full_path = self.repo_root.join(path);
            if !full_path.exists() {
                return Err(anyhow::anyhow!("Path does not exist: {}", path));
            }
            full_path
        } else {
            self.repo_root.as_path().to_path_buf()
        };

        let mut result = GrepResult::new(grep_args.pattern.clone());

        if search_path.is_file() {
            self.search_file(&search_path, &regex, &grep_args, &mut result)
                .await?;
        } else {
            self.search_directory(&search_path, &regex, &grep_args, &mut result)
                .await?;
        }

        Ok(result.to_json())
    }
}
