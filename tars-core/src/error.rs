//! Error types for the Tars framework

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TarsError {
    #[error("Codec error: {0}")]
    Codec(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Transport error: {0}")]
    Transport(#[from] std::io::Error),

    #[error("Timeout error: operation timed out after {0}ms")]
    Timeout(u64),

    #[error("No available endpoint")]
    NoEndpoint,

    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Server error: code={code}, message={message}")]
    ServerError { code: i32, message: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Queue full")]
    QueueFull,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

pub type Result<T> = std::result::Result<T, TarsError>;
