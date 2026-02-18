use thiserror::Error;

#[derive(Debug, Error)]
pub enum CreateFileError {
    #[error("Path is outside workspace: {0}")]
    PathOutsideWorkspace(String),

    #[error("Failed to create parent directories: {0}")]
    CreateDirsFailed(String),

    #[error("Failed to write file: {0}")]
    WriteFailed(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
