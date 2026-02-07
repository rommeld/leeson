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
    /// A configuration file could not be found, read, or deserialized.
    #[error("configuration error: {0}")]
    Config(#[from] config::ConfigError),

    /// A WebSocket operation (connect, send, receive) failed.
    #[error("websocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),

    /// JSON serialization or deserialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
