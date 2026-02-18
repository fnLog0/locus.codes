mod args;
mod error;

pub use args::{FinderArgs, FinderResult, SearchMatch};
pub use error::FinderError;

use crate::tools::{Glob, Grep, GrepArgs, Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::Arc;

// TODO: Future Enhancements for Intelligent Search
// ================================================
// 1. Semantic Search: Add embedding-based search using vector databases (e.g., LanceDB, Qdrant)
//    - Convert code to embeddings for natural language queries
//    - Allow queries like "find authentication code" without exact text match
//    - Consider models: VoyageCode3, OpenAI text-embedding-3, Jina Code Embeddings V2
//
// 2. Symbol Search: AST-based code intelligence using tree-sitter
//    - Find functions, classes, structs, enums, traits by name
//    - Support cross-file symbol resolution
//    - Add "go-to-definition" and "find-references" capabilities
//
// 3. LSP Integration: Leverage Language Server Protocol for deep code understanding
//    - Real-time symbol indexing
//    - Type-aware search
//    - Rename refactoring support
//    - Hover documentation

pub struct Finder {
    repo_root: Arc<std::path::PathBuf>,
    glob: Glob,
    grep: Grep,
}

impl Finder {
    pub fn new(repo_root: std::path::PathBuf) -> Self {
        let repo_root_arc = Arc::new(repo_root.clone());
        Self {
            repo_root: Arc::clone(&repo_root_arc),
            glob: Glob::new(repo_root.clone()),
            grep: Grep::new(repo_root),
        }
    }

    fn get_file_extensions(&self, file_type: &str) -> Vec<&'static str> {
        match file_type.to_lowercase().as_str() {
            "rust" | "rs" => vec![".rs"],
            "javascript" | "js" => vec![".js", ".jsx", ".mjs", ".cjs"],
            "typescript" | "ts" => vec![".ts", ".tsx", ".mts", ".cts"],
            "python" | "py" => vec![".py", ".pyi", ".pyw"],
            "go" => vec![".go"],
            "java" => vec![".java"],
            "c" => vec![".c", ".h"],
            "cpp" | "c++" => vec![".cpp", ".hpp", ".cc", ".cxx", ".hxx"],
            "ruby" | "rb" => vec![".rb", ".rake"],
            "php" => vec![".php"],
            "swift" => vec![".swift"],
            "kotlin" | "kt" => vec![".kt", ".kts"],
            "scala" => vec![".scala", ".sc"],
            "html" => vec![".html", ".htm"],
            "css" => vec![".css", ".scss", ".sass", ".less"],
            "json" => vec![".json"],
            "yaml" | "yml" => vec![".yaml", ".yml"],
            "markdown" | "md" => vec![".md", ".markdown"],
            "shell" | "sh" | "bash" => vec![".sh", ".bash", ".zsh"],
            "sql" => vec![".sql"],
            "toml" => vec![".toml"],
            "config" => vec![".config", ".conf", ".cfg", ".ini"],
            _ => vec![],
        }
    }

    fn build_glob_pattern(&self, args: &FinderArgs) -> Option<String> {
        if let Some(ref pattern) = args.file_pattern {
            return Some(pattern.clone());
        }

        if let Some(ref file_type) = args.file_type {
            let extensions = self.get_file_extensions(file_type);
            if !extensions.is_empty() {
                return Some(format!("**/*{}", extensions[0]));
            }
        }

        None
    }

    async fn get_files_to_search(
        &self,
        args: &FinderArgs,
    ) -> Result<Vec<std::path::PathBuf>, FinderError> {
        // If file_pattern or file_type specified, use glob
        if let Some(glob_pattern) = self.build_glob_pattern(args) {
            let mut glob_args = serde_json::json!({
                "pattern": glob_pattern,
                "max_results": 10000
            });

            if let Some(ref path) = args.path {
                glob_args["path"] = serde_json::json!(path);
            }

            if !args.exclude.is_empty() {
                glob_args["exclude"] = serde_json::json!(args.exclude);
            }

            let glob_result = self
                .glob
                .execute(glob_args)
                .await
                .map_err(|e| FinderError::SearchError(e.to_string()))?;

            let files = glob_result
                .get("files")
                .and_then(|f| f.as_array())
                .ok_or_else(|| FinderError::SearchError("Invalid glob result".to_string()))?;

            let file_paths: Vec<std::path::PathBuf> = files
                .iter()
                .filter_map(|f| f.as_str())
                .map(|f| self.repo_root.join(f))
                .collect();

            return Ok(file_paths);
        }

        // Fallback: use glob with catch-all pattern
        let mut glob_args = serde_json::json!({
            "pattern": "**/*",
            "max_results": 10000,
            "include_dirs": false
        });

        if let Some(ref path) = args.path {
            glob_args["path"] = serde_json::json!(path);
        }

        if !args.exclude.is_empty() {
            glob_args["exclude"] = serde_json::json!(args.exclude);
        }

        let glob_result = self
            .glob
            .execute(glob_args)
            .await
            .map_err(|e| FinderError::SearchError(e.to_string()))?;

        let files = glob_result
            .get("files")
            .and_then(|f| f.as_array())
            .ok_or_else(|| FinderError::SearchError("Invalid glob result".to_string()))?;

        let file_paths: Vec<std::path::PathBuf> = files
            .iter()
            .filter_map(|f| f.as_str())
            .map(|f| self.repo_root.join(f))
            .collect();

        Ok(file_paths)
    }
}

#[async_trait]
impl Tool for Finder {
    fn name(&self) -> &'static str {
        "finder"
    }

    fn description(&self) -> &'static str {
        "Intelligently search codebase for patterns. Supports literal text, regex, file filtering by type/pattern, and context lines. Internally uses glob for file discovery and grep for text search."
    }

    fn parameters_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query (literal text or regex pattern)"
                },
                "path": {
                    "type": "string",
                    "description": "Relative path to search within (optional, defaults to repo root)"
                },
                "file_pattern": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g., '*.rs', '**/test/**')"
                },
                "file_type": {
                    "type": "string",
                    "description": "Filter by file type: rust, javascript, typescript, python, go, java, c, cpp, ruby, php, swift, kotlin, scala, html, css, json, yaml, markdown, shell, sql, toml, config",
                    "enum": ["rust", "javascript", "typescript", "python", "go", "java", "c", "cpp", "ruby", "php", "swift", "kotlin", "scala", "html", "css", "json", "yaml", "markdown", "shell", "sql", "toml", "config"]
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Whether the search should be case sensitive (default: false)",
                    "default": false
                },
                "regex": {
                    "type": "boolean",
                    "description": "Treat query as a regex pattern (default: false, treats as literal)",
                    "default": false
                },
                "context_lines": {
                    "type": "integer",
                    "description": "Number of context lines before and after match (default: 3)",
                    "default": 3
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 100)",
                    "default": 100
                },
                "exclude": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Paths or patterns to exclude from search"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: JsonValue) -> ToolResult {
        let finder_args: FinderArgs = serde_json::from_value(args)?;

        // Get files using glob
        let files = self.get_files_to_search(&finder_args).await?;

        // Use grep for text search
        let grep_args = GrepArgs {
            pattern: finder_args.query.clone(),
            path: None, // We pass files directly
            case_sensitive: finder_args.case_sensitive,
            regex: finder_args.regex,
            context_lines: finder_args.context_lines,
            max_results: finder_args.max_results,
            files_only: false,
        };

        let grep_result = self
            .grep
            .search_with_files(&files, grep_args)
            .await
            .map_err(|e| FinderError::SearchError(e.to_string()))?;

        // Convert GrepResult to FinderResult
        let result = FinderResult {
            query: finder_args.query,
            matches: grep_result
                .matches
                .into_iter()
                .map(|m| SearchMatch {
                    file: m.file,
                    line_number: m.line_number,
                    column: m.column,
                    line: m.line,
                    context_before: m.context_before,
                    context_after: m.context_after,
                    match_start: m.match_start,
                    match_end: m.match_end,
                })
                .collect(),
            files_searched: grep_result.files_with_matches,
            total_matches: grep_result.total_matches,
            truncated: grep_result.truncated,
        };

        Ok(result.to_json())
    }
}
