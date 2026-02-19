use thiserror::Error;

#[derive(Debug, Error)]
pub enum HandoffError {
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),
}
