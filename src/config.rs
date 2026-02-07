use config::Config;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AppConfig {
    pub kraken: KrakenConfig,
}

#[derive(Deserialize)]
pub struct KrakenConfig {
    pub websocket_url: String,
}

pub fn fetch_config() -> Result<AppConfig, config::ConfigError> {
    let config = Config::builder()
        .add_source(config::File::with_name("./config/config.toml").required(true))
        .build()?;

    config.try_deserialize::<AppConfig>()
}
