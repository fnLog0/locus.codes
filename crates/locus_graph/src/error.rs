//! Error types for LocusGraph operations.

use thiserror::Error;

/// Errors that can occur when working with LocusGraph.
#[derive(Error, Debug)]
pub enum LocusGraphError {
    /// Configuration error (missing env vars, invalid values)
    #[error("Configuration error: {0}")]
    Config(String),

    /// Error from the underlying proxy client
    #[error("Proxy error: {0}")]
    Proxy(#[from] locus_proxy::LocusProxyError),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Generic error with message
    #[error("{0}")]
    Other(String),
}

/// Result type for LocusGraph operations.
pub type Result<T> = std::result::Result<T, LocusGraphError>;
