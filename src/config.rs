//! Application configuration loaded from environment variables.
//!
//! Credentials **must** be provided via environment variables:
//! - `KRAKEN_API_KEY` — API key for Kraken authentication
//! - `KRAKEN_API_SECRET` — API secret for Kraken authentication
//!
//! An optional `KRAKEN_WEBSOCKET_URL` overrides the default public endpoint.

use std::fmt;

use zeroize::Zeroizing;

/// Default public WebSocket endpoint.
const DEFAULT_WEBSOCKET_URL: &str = "wss://ws.kraken.com/v2";

/// Top-level application configuration.
#[derive(Debug)]
pub struct AppConfig {
    pub kraken: KrakenConfig,
    /// When true, orders are simulated locally instead of sent to the exchange.
    pub simulation: bool,
}

/// Kraken-specific configuration values.
///
/// Credentials are wrapped in [`Zeroizing`] so the backing memory is
/// overwritten with zeros when the value is dropped.
pub struct KrakenConfig {
    pub websocket_url: String,
    pub api_key: Option<Zeroizing<String>>,
    pub api_secret: Option<Zeroizing<String>>,
}

impl fmt::Debug for KrakenConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KrakenConfig")
            .field("websocket_url", &self.websocket_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field(
                "api_secret",
                &self.api_secret.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
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
    let websocket_url =
        non_empty_var("KRAKEN_WEBSOCKET_URL").unwrap_or_else(|| DEFAULT_WEBSOCKET_URL.to_string());

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

    let simulation = non_empty_var("LEESON_SIMULATION").is_some_and(|v| v == "true" || v == "1");

    Ok(AppConfig {
        kraken: KrakenConfig {
            websocket_url,
            api_key: api_key.map(Zeroizing::new),
            api_secret: api_secret.map(Zeroizing::new),
        },
        simulation,
    })
}

/// Returns the value of an environment variable if it exists and is non-empty.
fn non_empty_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Mutex to serialize tests that mutate environment variables.
    /// Rust runs tests in parallel, so without this lock the `with_env`
    /// helper races across threads.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Helper that temporarily sets env vars, runs `f`, then restores originals.
    ///
    /// Acquires `ENV_LOCK` to prevent concurrent env var mutations.
    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");

        let originals: Vec<(&str, Option<String>)> = vars
            .iter()
            .map(|(k, _)| (*k, std::env::var(k).ok()))
            .collect();

        for (k, v) in vars {
            // SAFETY: serialized by ENV_LOCK — no other test thread touches
            // these env vars while this guard is held.
            unsafe {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }

        f();

        for (k, original) in originals {
            // SAFETY: restoring original values, same serialized context.
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
                assert_eq!(
                    config.kraken.api_key.as_deref().map(String::as_str),
                    Some("test-key")
                );
                assert_eq!(
                    config.kraken.api_secret.as_deref().map(String::as_str),
                    Some("test-secret")
                );
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
    fn simulation_mode_from_env() {
        with_env(
            &[
                ("KRAKEN_API_KEY", None),
                ("KRAKEN_API_SECRET", None),
                ("KRAKEN_WEBSOCKET_URL", None),
                ("LEESON_SIMULATION", Some("true")),
            ],
            || {
                let config = fetch_config().unwrap();
                assert!(config.simulation);
            },
        );
    }

    #[test]
    fn simulation_mode_accepts_1() {
        with_env(
            &[
                ("KRAKEN_API_KEY", None),
                ("KRAKEN_API_SECRET", None),
                ("KRAKEN_WEBSOCKET_URL", None),
                ("LEESON_SIMULATION", Some("1")),
            ],
            || {
                let config = fetch_config().unwrap();
                assert!(config.simulation);
            },
        );
    }

    #[test]
    fn simulation_mode_defaults_to_off() {
        with_env(
            &[
                ("KRAKEN_API_KEY", None),
                ("KRAKEN_API_SECRET", None),
                ("KRAKEN_WEBSOCKET_URL", None),
                ("LEESON_SIMULATION", None),
            ],
            || {
                let config = fetch_config().unwrap();
                assert!(!config.simulation);
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
