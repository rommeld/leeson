//! Application configuration loaded from `config/config.toml`.

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
}

/// Loads and deserializes the application configuration from disk.
///
/// # Errors
///
/// Returns [`config::ConfigError`] if the configuration file is missing,
/// malformed, or cannot be deserialized into [`AppConfig`].
pub fn fetch_config() -> Result<AppConfig, config::ConfigError> {
    let config = Config::builder()
        .add_source(config::File::with_name("./config/config.toml").required(true))
        .build()?;

    config.try_deserialize::<AppConfig>()
}
