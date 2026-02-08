//! Shared test utilities and constants.

use std::sync::Arc;

/// Kraken WebSocket V2 public endpoint URL.
pub const KRAKEN_WS_URL: &str = "wss://ws.kraken.com/v2";

/// Builds a rustls TLS config with the pinned Kraken CA for use in tests.
pub fn test_tls_config() -> Arc<rustls::ClientConfig> {
    Arc::new(leeson::tls::build_tls_config().expect("failed to build TLS config"))
}
