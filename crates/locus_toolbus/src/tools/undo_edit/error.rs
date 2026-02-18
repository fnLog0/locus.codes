use thiserror::Error;

#[derive(Debug, Error)]
pub enum UndoEditError {
    #[error("Path is outside workspace: {0}")]
    PathOutsideWorkspace(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Nothing to undo for this file")]
    NothingToUndo,

    #[error("Invalid path: {0}")]
    InvalidPath(String),
}
