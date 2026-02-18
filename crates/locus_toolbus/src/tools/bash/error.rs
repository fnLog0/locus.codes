use thiserror::Error;

#[derive(Debug, Error)]
pub enum BashError {
    #[error("Command timed out after {0} seconds")]
    Timeout(u64),

    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Failed to spawn command: {0}")]
    SpawnFailed(String),

    #[error("Failed to wait for command: {0}")]
    WaitFailed(String),

    #[error("UTF-8 decode error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
