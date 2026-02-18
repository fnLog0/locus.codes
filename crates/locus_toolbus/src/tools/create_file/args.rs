use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateFileArgs {
    /// The path to the file to create or overwrite
    pub path: String,

    /// The content to write to the file
    pub content: String,

    /// Whether to create parent directories if they don't exist
    #[serde(default = "default_create_dirs")]
    pub create_dirs: bool,
}

fn default_create_dirs() -> bool {
    true
}
