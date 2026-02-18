use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct EditFileArgs {
    /// The path to the file to edit
    pub path: String,

    /// The text to find and replace
    pub old_string: String,

    /// The replacement text
    pub new_string: String,

    /// Replace all occurrences (default: false, only replace first)
    #[serde(default)]
    pub replace_all: bool,
}
