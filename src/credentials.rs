//! Secure credential storage via the macOS Keychain.
//!
//! Provides functions to load, save, and check API keys stored in
//! the system keychain. At startup, [`populate_env_from_keychain`]
//! copies any stored credentials into environment variables so the
//! existing config flow picks them up transparently.

use tracing::{debug, warn};
use zeroize::Zeroizing;

/// Keychain service name used for all stored credentials.
const SERVICE: &str = "leeson";

/// Known API credential keys managed by this module.
#[derive(Clone, Copy, Debug)]
pub enum CredentialKey {
    FireworksApiKey,
    KrakenApiKey,
    KrakenApiSecret,
}

impl CredentialKey {
    /// Returns the keychain entry identifier.
    pub fn keyring_id(self) -> &'static str {
        match self {
            Self::FireworksApiKey => "fireworks_api_key",
            Self::KrakenApiKey => "kraken_api_key",
            Self::KrakenApiSecret => "kraken_api_secret",
        }
    }

    /// Returns the environment variable name for this credential.
    pub fn env_var(self) -> &'static str {
        match self {
            Self::FireworksApiKey => "FIREWORKS_API_KEY",
            Self::KrakenApiKey => "KRAKEN_API_KEY",
            Self::KrakenApiSecret => "KRAKEN_API_SECRET",
        }
    }

    /// Returns a human-readable label for TUI display.
    pub fn label(self) -> &'static str {
        match self {
            Self::FireworksApiKey => "Fireworks API Key",
            Self::KrakenApiKey => "Kraken API Key",
            Self::KrakenApiSecret => "Kraken API Secret",
        }
    }

    /// All credential keys in display order.
    pub const ALL: [CredentialKey; 3] = [
        Self::FireworksApiKey,
        Self::KrakenApiKey,
        Self::KrakenApiSecret,
    ];
}

/// Loads a credential from the keychain, returning `None` if not set.
pub fn load(key: CredentialKey) -> Option<Zeroizing<String>> {
    let entry = keyring::Entry::new(SERVICE, key.keyring_id()).ok()?;
    match entry.get_password() {
        Ok(password) => Some(Zeroizing::new(password)),
        Err(keyring::Error::NoEntry) => None,
        Err(e) => {
            warn!(key = key.keyring_id(), error = %e, "failed to read keychain entry");
            None
        }
    }
}

/// Saves a credential to the keychain.
pub fn save(key: CredentialKey, value: &str) -> crate::Result<()> {
    let entry = keyring::Entry::new(SERVICE, key.keyring_id())
        .map_err(|e| crate::LeesonError::Config(format!("keyring entry error: {e}")))?;
    entry
        .set_password(value)
        .map_err(|e| crate::LeesonError::Config(format!("failed to save to keychain: {e}")))
}

/// Checks whether a credential exists in the keychain.
pub fn is_set(key: CredentialKey) -> bool {
    load(key).is_some()
}

/// Populates environment variables from the keychain for any
/// credentials not already set in the environment.
///
/// Call this at startup before [`crate::config::fetch_config`].
pub fn populate_env_from_keychain() {
    for key in CredentialKey::ALL {
        if std::env::var(key.env_var()).is_err()
            && let Some(value) = load(key)
        {
            debug!(key = key.env_var(), "loaded credential from keychain");
            // SAFETY: single-threaded at this point (before tokio runtime starts tasks)
            unsafe {
                std::env::set_var(key.env_var(), value.as_str());
            }
        }
    }
}
