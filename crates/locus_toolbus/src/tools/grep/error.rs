use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrepError {
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(String),

    #[error("Path does not exist: {0}")]
    PathNotFound(String),

    #[error("Failed to read file: {0}")]
    ReadError(String),

    #[error("Empty pattern provided")]
    EmptyPattern,
}

impl From<regex::Error> for GrepError {
    fn from(err: regex::Error) -> Self {
        GrepError::InvalidRegex(err.to_string())
    }
}
