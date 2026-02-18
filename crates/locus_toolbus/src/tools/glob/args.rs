use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobArgs {
    pub pattern: String,

    #[serde(default)]
    pub path: Option<String>,

    #[serde(default = "default_max_results")]
    pub max_results: usize,

    #[serde(default)]
    pub include_dirs: bool,

    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_max_results() -> usize {
    1000
}

impl GlobArgs {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            path: None,
            max_results: 1000,
            include_dirs: false,
            exclude: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobResult {
    pub pattern: String,
    pub files: Vec<String>,
    pub total_matches: usize,
    pub truncated: bool,
}

impl GlobResult {
    pub fn new(pattern: String) -> Self {
        Self {
            pattern,
            files: Vec::new(),
            total_matches: 0,
            truncated: false,
        }
    }

    pub fn add_file(&mut self, file: String) {
        self.files.push(file);
        self.total_matches += 1;
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({ "error": "serialization failed" }))
    }
}
