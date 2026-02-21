//! Error types for observability crate

use thiserror::Error;

/// Errors that can occur during observability initialization or operation
#[derive(Error, Debug)]
pub enum ObservabilityError {
    /// Failed to initialize OpenTelemetry
    #[error("Failed to initialize observability: {0}")]
    InitFailed(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}
