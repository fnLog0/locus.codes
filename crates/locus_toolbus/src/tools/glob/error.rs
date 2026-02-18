use thiserror::Error;

#[derive(Debug, Error)]
pub enum GlobError {
    #[error("Invalid glob pattern: {0}")]
    InvalidPattern(String),

    #[error("Path does not exist: {0}")]
    PathNotFound(String),

    #[error("Path is outside repository: {0}")]
    PathOutsideRepo(String),

    #[error("IO error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for GlobError {
    fn from(err: std::io::Error) -> Self {
        GlobError::IoError(err.to_string())
    }
}
