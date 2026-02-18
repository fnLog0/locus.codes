use thiserror::Error;

#[derive(Debug, Error)]
pub enum FinderError {
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(String),

    #[error("Invalid glob pattern: {0}")]
    InvalidGlob(String),

    #[error("Path does not exist: {0}")]
    PathNotFound(String),

    #[error("Path is outside repository: {0}")]
    PathOutsideRepo(String),

    #[error("Failed to read file: {0}")]
    ReadError(String),

    #[error("Search error: {0}")]
    SearchError(String),

    #[error("Empty query provided")]
    EmptyQuery,
}

impl From<regex::Error> for FinderError {
    fn from(err: regex::Error) -> Self {
        FinderError::InvalidRegex(err.to_string())
    }
}

impl From<glob::PatternError> for FinderError {
    fn from(err: glob::PatternError) -> Self {
        FinderError::InvalidGlob(err.to_string())
    }
}
