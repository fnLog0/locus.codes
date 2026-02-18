use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FinderArgs {
    pub query: String,

    #[serde(default)]
    pub path: Option<String>,

    #[serde(default)]
    pub file_pattern: Option<String>,

    #[serde(default)]
    pub file_type: Option<String>,

    #[serde(default = "default_case_sensitive")]
    pub case_sensitive: bool,

    #[serde(default)]
    pub regex: bool,

    #[serde(default = "default_context_lines")]
    pub context_lines: usize,

    #[serde(default = "default_max_results")]
    pub max_results: usize,

    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_case_sensitive() -> bool {
    false
}

fn default_context_lines() -> usize {
    3
}

fn default_max_results() -> usize {
    100
}

impl FinderArgs {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            path: None,
            file_pattern: None,
            file_type: None,
            case_sensitive: false,
            regex: false,
            context_lines: 3,
            max_results: 100,
            exclude: Vec::new(),
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_file_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.file_pattern = Some(pattern.into());
        self
    }

    pub fn with_file_type(mut self, file_type: impl Into<String>) -> Self {
        self.file_type = Some(file_type.into());
        self
    }

    pub fn case_sensitive(mut self, yes: bool) -> Self {
        self.case_sensitive = yes;
        self
    }

    pub fn with_regex(mut self, yes: bool) -> Self {
        self.regex = yes;
        self
    }

    pub fn with_context_lines(mut self, lines: usize) -> Self {
        self.context_lines = lines;
        self
    }

    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    pub fn with_exclude(mut self, patterns: Vec<String>) -> Self {
        self.exclude = patterns;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
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
pub struct FinderResult {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub files_searched: usize,
    pub total_matches: usize,
    pub truncated: bool,
}

impl FinderResult {
    pub fn new(query: String) -> Self {
        Self {
            query,
            matches: Vec::new(),
            files_searched: 0,
            total_matches: 0,
            truncated: false,
        }
    }

    pub fn add_match(&mut self, m: SearchMatch) {
        self.matches.push(m);
        self.total_matches += 1;
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({ "error": "serialization failed" }))
    }
}
