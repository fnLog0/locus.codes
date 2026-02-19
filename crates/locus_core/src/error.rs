use thiserror::Error;

#[derive(Error, Debug)]
pub enum LocusError {
    #[error("session error: {0}")]
    Session(String),

    #[error("tool error: {0}")]
    Tool(String),

    #[error("memory error: {0}")]
    Memory(String),

    #[error("config error: {0}")]
    Config(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, LocusError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_error() {
        let err = LocusError::Session("test session error".to_string());
        assert_eq!(err.to_string(), "session error: test session error");
    }

    #[test]
    fn test_tool_error() {
        let err = LocusError::Tool("bash failed".to_string());
        assert_eq!(err.to_string(), "tool error: bash failed");
    }

    #[test]
    fn test_memory_error() {
        let err = LocusError::Memory("recall failed".to_string());
        assert_eq!(err.to_string(), "memory error: recall failed");
    }

    #[test]
    fn test_config_error() {
        let err = LocusError::Config("invalid model".to_string());
        assert_eq!(err.to_string(), "config error: invalid model");
    }

    #[test]
    fn test_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = LocusError::from(io_err);
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json");
        let err = LocusError::from(json_err.unwrap_err());
        assert!(err.to_string().contains("expected value"));
    }
}
