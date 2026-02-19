use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebAutomationError {
    #[error("TinyFish API key not set. Set TINYFISH_API_KEY environment variable.")]
    MissingApiKey,

    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("API error: {code} - {message}")]
    ApiError { code: String, message: String },
}
