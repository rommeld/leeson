//! Configuration loading tests.

use std::env;
use std::path::PathBuf;

/// Helper to get the path to test fixtures directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn test_valid_config_deserializes() {
    let config_path = fixtures_dir().join("config.toml");
    let config_content =
        std::fs::read_to_string(&config_path).expect("Failed to read test config file");

    let config: toml::Value = toml::from_str(&config_content).expect("Failed to parse config TOML");

    assert!(config.get("kraken").is_some());
    assert_eq!(
        config["kraken"]["websocket_url"].as_str(),
        Some("wss://ws.kraken.com/v2")
    );
}

#[test]
fn test_invalid_config_missing_field() {
    let config_path = fixtures_dir().join("invalid_config.toml");
    let config_content =
        std::fs::read_to_string(&config_path).expect("Failed to read test config file");

    let config: toml::Value = toml::from_str(&config_content).expect("Failed to parse config TOML");

    // The invalid config has kraken section but no websocket_url
    assert!(config.get("kraken").is_some());
    assert!(config["kraken"].get("websocket_url").is_none());
}

#[test]
fn test_config_file_not_found() {
    let nonexistent_path = fixtures_dir().join("nonexistent.toml");
    let result = std::fs::read_to_string(&nonexistent_path);

    assert!(result.is_err());
}
