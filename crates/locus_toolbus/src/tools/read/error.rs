use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("Path is outside workspace: {0}")]
    PathOutsideWorkspace(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Not a file: {0}")]
    NotAFile(String),

    #[error("Not a directory: {0}")]
    NotADirectory(String),

    #[error("File is not valid UTF-8 (binary file)")]
    NotUtf8,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
