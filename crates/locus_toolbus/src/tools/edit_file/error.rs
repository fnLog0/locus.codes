use thiserror::Error;

#[derive(Debug, Error)]
pub enum EditFileError {
    #[error("Path is outside workspace: {0}")]
    PathOutsideWorkspace(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Old string not found in file")]
    OldStringNotFound,

    #[error("Multiple matches found but replace_all is false")]
    MultipleMatches,

    #[error("Failed to read file: {0}")]
    ReadFailed(String),

    #[error("Failed to write file: {0}")]
    WriteFailed(String),

    #[error("Failed to create parent directories: {0}")]
    CreateDirsFailed(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("Edit {edit_number}: old string not found in file")]
    MultieditStringNotFound { edit_number: usize },

    #[error("Edit {edit_number}: multiple matches found but replace_all is false")]
    MultieditMultipleMatches { edit_number: usize },
}
