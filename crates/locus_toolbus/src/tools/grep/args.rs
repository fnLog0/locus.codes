use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrepArgs {
    pub pattern: String,

    #[serde(default)]
    pub path: Option<String>,

    #[serde(default = "default_case_sensitive")]
    pub case_sensitive: bool,

    #[serde(default)]
    pub regex: bool,

    #[serde(default = "default_context_lines")]
    pub context_lines: usize,

    #[serde(default = "default_max_results")]
    pub max_results: usize,

    #[serde(default)]
    pub files_only: bool,
}

fn default_case_sensitive() -> bool {
    false
}

fn default_context_lines() -> usize {
    2
}

fn default_max_results() -> usize {
    100
}

impl GrepArgs {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            path: None,
            case_sensitive: false,
            regex: false,
            context_lines: 2,
            max_results: 100,
            files_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    pub file: String,
    pub line_number: usize,
    pub column: usize,
    pub line: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    pub match_start: usize,
    pub match_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepResult {
    pub pattern: String,
    pub matches: Vec<GrepMatch>,
    pub files_with_matches: usize,
    pub total_matches: usize,
    pub truncated: bool,
}

impl GrepResult {
    pub fn new(pattern: String) -> Self {
        Self {
            pattern,
            matches: Vec::new(),
            files_with_matches: 0,
            total_matches: 0,
            truncated: false,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({ "error": "serialization failed" }))
    }
}
