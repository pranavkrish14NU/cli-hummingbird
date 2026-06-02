use thiserror::Error;

#[derive(Debug, Error)]
pub enum HummingbirdError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Inference error: {0}")]
    Inference(String),

    #[error("Context error: {0}")]
    Context(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Token limit exceeded: requested {requested}, limit {limit}")]
    TokenLimitExceeded { requested: usize, limit: usize },
}

pub type Result<T> = std::result::Result<T, HummingbirdError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_display() {
        let err = HummingbirdError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn inference_error_display() {
        let err = HummingbirdError::Inference("model timeout".to_string());
        assert_eq!(err.to_string(), "Inference error: model timeout");
    }

    #[test]
    fn config_error_display() {
        let err = HummingbirdError::Config("missing api_key".to_string());
        assert_eq!(err.to_string(), "Config error: missing api_key");
    }

    #[test]
    fn token_limit_error_display() {
        let err = HummingbirdError::TokenLimitExceeded { requested: 8000, limit: 4096 };
        assert!(err.to_string().contains("8000"));
        assert!(err.to_string().contains("4096"));
    }

    #[test]
    fn result_alias_works() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);
    }
}
