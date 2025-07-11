use redis::RedisError;
use serde_json;
use std::fmt;

#[derive(Debug)]
pub enum ConsumerError {
    Redis(RedisError),
    Serialization(serde_json::Error),
    Http(reqwest::Error),
    ConnectionFailed(String),
    InvalidMessage(String),
}

impl fmt::Display for ConsumerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsumerError::Redis(err) => write!(f, "Redis error: {}", err),
            ConsumerError::Serialization(err) => write!(f, "Serialization error: {}", err),
            ConsumerError::Http(err) => write!(f, "HTTP error: {}", err),
            ConsumerError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            ConsumerError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
        }
    }
}

impl std::error::Error for ConsumerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConsumerError::Redis(err) => Some(err),
            ConsumerError::Serialization(err) => Some(err),
            ConsumerError::Http(err) => Some(err),
            _ => None,
        }
    }
}

impl From<RedisError> for ConsumerError {
    fn from(err: RedisError) -> Self {
        ConsumerError::Redis(err)
    }
}

impl From<serde_json::Error> for ConsumerError {
    fn from(err: serde_json::Error) -> Self {
        ConsumerError::Serialization(err)
    }
}

impl From<reqwest::Error> for ConsumerError {
    fn from(err: reqwest::Error) -> Self {
        ConsumerError::Http(err)
    }
}

pub type ConsumerResult<T> = Result<T, ConsumerError>;
