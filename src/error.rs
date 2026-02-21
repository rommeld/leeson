//! Crate-level error types.
//!
//! [`LeesonError`] unifies every error source (configuration, WebSocket,
//! JSON) behind a single enum so callers can match on the variant they
//! care about while still using the `?` operator for easy propagation.

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, LeesonError>;

/// Top-level error type returned by all public APIs.
#[derive(Debug, thiserror::Error)]
pub enum LeesonError {
    /// A required environment variable is missing or empty.
    #[error("configuration error: {0}")]
    Config(String),

    /// A WebSocket operation (connect, send, receive) failed.
    #[error("websocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),

    /// JSON serialization or deserialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// A received WebSocket message could not be parsed as valid JSON.
    #[error("malformed message: {0}")]
    MalformedMessage(String),

    /// An HTTP request (e.g. fetching a WebSocket token) failed.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// A TLS configuration error occurred (e.g. invalid certificate).
    #[error("tls error: {0}")]
    Tls(String),

    /// An I/O operation (e.g. terminal setup) failed.
    #[error("io error: {0}")]
    Io(String),

    /// A channel send operation failed.
    #[error("channel error: {0}")]
    Channel(String),
}
