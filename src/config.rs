//! Application configuration loaded from environment variables.
//!
//! Credentials **must** be provided via environment variables:
//! - `KRAKEN_API_KEY` — API key for Kraken authentication
//! - `KRAKEN_API_SECRET` — API secret for Kraken authentication
//!
//! An optional `KRAKEN_WEBSOCKET_URL` overrides the default public endpoint.

/// Default public WebSocket endpoint.
const DEFAULT_WEBSOCKET_URL: &str = "wss://ws.kraken.com/v2";

/// Top-level application configuration.
#[derive(Debug)]
pub struct AppConfig {
    pub kraken: KrakenConfig,
}

/// Kraken-specific configuration values.
#[derive(Debug)]
pub struct KrakenConfig {
    pub websocket_url: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
}

/// Loads the application configuration from environment variables.
///
/// The WebSocket URL defaults to `wss://ws.kraken.com/v2` and can be
/// overridden with `KRAKEN_WEBSOCKET_URL`. API credentials are optional
/// (unauthenticated mode) but when one is set both must be present.
///
/// # Errors
///
/// Returns [`LeesonError::Config`](crate::LeesonError::Config) if only
/// one of the two credential variables is set.
pub fn fetch_config() -> crate::Result<AppConfig> {
    let websocket_url = non_empty_var("KRAKEN_WEBSOCKET_URL")
        .unwrap_or_else(|| DEFAULT_WEBSOCKET_URL.to_string());

    let api_key = non_empty_var("KRAKEN_API_KEY");
    let api_secret = non_empty_var("KRAKEN_API_SECRET");

    match (&api_key, &api_secret) {
        (Some(_), None) => {
            return Err(crate::LeesonError::Config(
                "KRAKEN_API_KEY is set but KRAKEN_API_SECRET is missing".to_string(),
            ));
        }
        (None, Some(_)) => {
            return Err(crate::LeesonError::Config(
                "KRAKEN_API_SECRET is set but KRAKEN_API_KEY is missing".to_string(),
            ));
        }
        _ => {}
    }

    Ok(AppConfig {
        kraken: KrakenConfig {
            websocket_url,
            api_key,
            api_secret,
        },
    })
}

/// Returns the value of an environment variable if it exists and is non-empty.
fn non_empty_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper that temporarily sets env vars, runs `f`, then restores originals.
    ///
    /// # Safety
    ///
    /// Tests using this helper must run with `--test-threads=1` or otherwise
    /// ensure no other threads read these env vars concurrently.
    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        let originals: Vec<(&str, Option<String>)> = vars
            .iter()
            .map(|(k, _)| (*k, std::env::var(k).ok()))
            .collect();

        for (k, v) in vars {
            // SAFETY: config tests run single-threaded (see test runner config).
            unsafe {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }

        f();

        for (k, original) in originals {
            // SAFETY: restoring original values, same single-threaded context.
            unsafe {
                match original {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    #[test]
    fn defaults_without_env_vars() {
        with_env(
            &[
                ("KRAKEN_API_KEY", None),
                ("KRAKEN_API_SECRET", None),
                ("KRAKEN_WEBSOCKET_URL", None),
            ],
            || {
                let config = fetch_config().unwrap();
                assert_eq!(config.kraken.websocket_url, DEFAULT_WEBSOCKET_URL);
                assert!(config.kraken.api_key.is_none());
                assert!(config.kraken.api_secret.is_none());
            },
        );
    }

    #[test]
    fn loads_credentials_from_env() {
        with_env(
            &[
                ("KRAKEN_API_KEY", Some("test-key")),
                ("KRAKEN_API_SECRET", Some("test-secret")),
                ("KRAKEN_WEBSOCKET_URL", None),
            ],
            || {
                let config = fetch_config().unwrap();
                assert_eq!(config.kraken.api_key.as_deref(), Some("test-key"));
                assert_eq!(config.kraken.api_secret.as_deref(), Some("test-secret"));
            },
        );
    }

    #[test]
    fn custom_websocket_url() {
        with_env(
            &[
                ("KRAKEN_API_KEY", None),
                ("KRAKEN_API_SECRET", None),
                ("KRAKEN_WEBSOCKET_URL", Some("wss://custom.example.com")),
            ],
            || {
                let config = fetch_config().unwrap();
                assert_eq!(config.kraken.websocket_url, "wss://custom.example.com");
            },
        );
    }

    #[test]
    fn rejects_key_without_secret() {
        with_env(
            &[
                ("KRAKEN_API_KEY", Some("key-only")),
                ("KRAKEN_API_SECRET", None),
            ],
            || {
                let err = fetch_config().unwrap_err();
                assert!(err.to_string().contains("KRAKEN_API_SECRET is missing"));
            },
        );
    }

    #[test]
    fn rejects_secret_without_key() {
        with_env(
            &[
                ("KRAKEN_API_KEY", None),
                ("KRAKEN_API_SECRET", Some("secret-only")),
            ],
            || {
                let err = fetch_config().unwrap_err();
                assert!(err.to_string().contains("KRAKEN_API_KEY is missing"));
            },
        );
    }

    #[test]
    fn empty_values_treated_as_absent() {
        with_env(
            &[
                ("KRAKEN_API_KEY", Some("")),
                ("KRAKEN_API_SECRET", Some("")),
                ("KRAKEN_WEBSOCKET_URL", Some("")),
            ],
            || {
                let config = fetch_config().unwrap();
                assert_eq!(config.kraken.websocket_url, DEFAULT_WEBSOCKET_URL);
                assert!(config.kraken.api_key.is_none());
                assert!(config.kraken.api_secret.is_none());
            },
        );
    }
}
