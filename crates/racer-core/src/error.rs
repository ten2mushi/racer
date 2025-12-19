use thiserror::Error;

use crate::validation::ValidationError;

#[derive(Debug, Error)]
pub enum RacerError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("operation timed out: {0}")]
    Timeout(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl RacerError {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn crypto(msg: impl Into<String>) -> Self {
        Self::Crypto(msg.into())
    }

    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }

    pub fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, RacerError>;
