use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct EditFileArgs {
    /// The path to the file to edit
    pub path: String,

    /// The text to find and replace. If empty or omitted, overwrites the entire file.
    #[serde(default)]
    pub old_string: Option<String>,

    /// The replacement text (or full file content if old_string is empty)
    #[serde(default)]
    pub new_string: Option<String>,

    /// Replace all occurrences (default: false, only replace first)
    #[serde(default)]
    pub replace_all: bool,

    /// Array of edit operations. If provided, old_string/new_string/replace_all are ignored.
    #[serde(default)]
    pub edits: Option<Vec<EditOperation>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EditOperation {
    /// The text to find and replace
    pub old_string: String,

    /// The replacement text
    pub new_string: String,

    /// Replace all occurrences (default: false, only replace first)
    #[serde(default)]
    pub replace_all: bool,
}
