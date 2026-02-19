use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ReadArgs {
    /// Path to the file or directory (relative to repo root)
    pub path: String,

    /// Maximum bytes to read from a file (default: 1MB). Ignored when listing a directory.
    #[serde(default = "default_max_bytes")]
    pub max_bytes: u64,
}

fn default_max_bytes() -> u64 {
    1_048_576 // 1 MiB
}
