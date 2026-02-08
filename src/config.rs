//! Application configuration loaded from `config/config.toml`.
//!
//! Credentials can be provided via environment variables or the config file.
//! Environment variables take precedence:
//! - `KRAKEN_API_KEY` - API key for Kraken authentication
//! - `KRAKEN_API_SECRET` - API secret for Kraken authentication

use config::Config;
use serde::Deserialize;

/// Top-level application configuration.
#[derive(Deserialize)]
pub struct AppConfig {
    pub kraken: KrakenConfig,
}

/// Kraken-specific configuration values.
#[derive(Deserialize)]
pub struct KrakenConfig {
    pub websocket_url: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
}

/// Raw configuration as read from the config file.
#[derive(Deserialize)]
struct RawAppConfig {
    kraken: RawKrakenConfig,
}

/// Raw Kraken configuration before environment variable overlay.
#[derive(Deserialize)]
struct RawKrakenConfig {
    websocket_url: String,
    api_key: Option<String>,
    api_secret: Option<String>,
}

/// Loads and deserializes the application configuration from disk.
///
/// Credentials are resolved with the following precedence:
/// 1. Environment variables (`KRAKEN_API_KEY`, `KRAKEN_API_SECRET`)
/// 2. Config file values
///
/// # Errors
///
/// Returns [`LeesonError::Config`](crate::LeesonError::Config) if the
/// configuration file is missing, malformed, or cannot be deserialized
/// into [`AppConfig`].
pub fn fetch_config() -> crate::Result<AppConfig> {
    let config = Config::builder()
        .add_source(config::File::with_name("./config/config.toml").required(true))
        .build()?;

    let raw: RawAppConfig = config.try_deserialize()?;

    // Environment variables take precedence over config file
    let api_key = std::env::var("KRAKEN_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .or(raw.kraken.api_key);

    let api_secret = std::env::var("KRAKEN_API_SECRET")
        .ok()
        .filter(|s| !s.is_empty())
        .or(raw.kraken.api_secret);

    Ok(AppConfig {
        kraken: KrakenConfig {
            websocket_url: raw.kraken.websocket_url,
            api_key,
            api_secret,
        },
    })
}
