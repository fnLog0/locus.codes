use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UndoEditArgs {
    /// Path to the file to undo (relative to workspace root). Required.
    pub path: String,
}
