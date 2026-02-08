//! TLS configuration with a pinned CA root certificate.
//!
//! Builds a [`rustls::ClientConfig`] that trusts only the GTS Root R4
//! certificate authority, which signs the TLS chain for both
//! `ws.kraken.com` and `api.kraken.com`.

use rustls::ClientConfig;

use crate::Result;

/// GTS Root R4 PEM, embedded at compile time.
static GTS_ROOT_R4_PEM: &[u8] = include_bytes!("../certs/gts_root_r4.pem");

/// Builds a [`ClientConfig`] whose root store contains only the pinned
/// GTS Root R4 CA certificate.
///
/// # Errors
///
/// Returns [`LeesonError::Tls`](crate::LeesonError::Tls) if the embedded
/// PEM cannot be parsed.
pub fn build_tls_config() -> Result<ClientConfig> {
    let mut root_store = rustls::RootCertStore::empty();

    let certs: Vec<_> = rustls_pemfile::certs(&mut &GTS_ROOT_R4_PEM[..])
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| crate::LeesonError::Tls(format!("failed to parse CA PEM: {e}")))?;

    root_store.add_parsable_certificates(certs);

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(config)
}
