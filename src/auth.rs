//! Kraken REST API authentication and WebSocket token retrieval.
//!
//! The Level-3 (orders) WebSocket channel requires an authentication
//! token obtained via the
//! [`GetWebSocketsToken`](https://docs.kraken.com/api/docs/rest-api/get-websockets-token)
//! REST endpoint.  The token is valid for 15 minutes after creation.

use base64::prelude::*;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

use crate::Result;

const TOKEN_URL: &str = "https://api.kraken.com/0/private/GetWebSocketsToken";
const URL_PATH: &str = "/0/private/GetWebSocketsToken";

/// Fetches a short-lived WebSocket authentication token from the Kraken REST API.
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if the HTTP request fails,
/// the response cannot be parsed, or the API returns an error.
pub async fn get_websocket_token(api_key: &str, api_secret: &str) -> Result<String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_millis();

    let post_data = format!("nonce={nonce}");
    let signature = sign(api_secret, URL_PATH, nonce, &post_data)?;

    let client = reqwest::Client::new();
    let response = client
        .post(TOKEN_URL)
        .header("API-Key", api_key)
        .header("API-Sign", &signature)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(post_data)
        .send()
        .await?;

    let body: serde_json::Value = response.json().await?;

    let errors = body["error"].as_array();
    if let Some(errors) = errors
        && !errors.is_empty()
    {
        let messages: Vec<String> = errors
            .iter()
            .filter_map(|e| e.as_str().map(String::from))
            .collect();
        return Err(crate::LeesonError::MalformedMessage(format!(
            "Kraken API error: {}",
            messages.join(", ")
        )));
    }

    let token = body["result"]["token"]
        .as_str()
        .ok_or_else(|| {
            crate::LeesonError::MalformedMessage(
                "missing token in GetWebSocketsToken response".into(),
            )
        })?
        .to_string();

    info!("Obtained WebSocket authentication token");
    Ok(token)
}

/// Computes the `API-Sign` header value.
///
/// Algorithm: `Base64(HMAC-SHA512(Base64Decode(secret), path + SHA256(nonce + post_data)))`
fn sign(api_secret: &str, path: &str, nonce: u128, post_data: &str) -> Result<String> {
    let secret = BASE64_STANDARD.decode(api_secret).map_err(|e| {
        crate::LeesonError::MalformedMessage(format!("invalid base64 api_secret: {e}"))
    })?;

    let mut sha256 = Sha256::new();
    sha256.update(format!("{nonce}{post_data}").as_bytes());
    let sha256_digest = sha256.finalize();

    let mut hmac_input = Vec::from(path.as_bytes());
    hmac_input.extend_from_slice(&sha256_digest);

    let mut mac = Hmac::<Sha512>::new_from_slice(&secret)
        .map_err(|e| crate::LeesonError::MalformedMessage(format!("invalid HMAC key: {e}")))?;
    mac.update(&hmac_input);
    let result = mac.finalize().into_bytes();

    Ok(BASE64_STANDARD.encode(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_produces_deterministic_output() {
        // Use a known base64-encoded secret (32 bytes of zeros).
        let secret = BASE64_STANDARD.encode([0u8; 32]);
        let nonce = 1_000_000_000_000u128;
        let post_data = "nonce=1000000000000";

        let sig1 = sign(&secret, URL_PATH, nonce, post_data).unwrap();
        let sig2 = sign(&secret, URL_PATH, nonce, post_data).unwrap();
        assert_eq!(sig1, sig2);

        // Verify the output is valid base64.
        assert!(BASE64_STANDARD.decode(&sig1).is_ok());
    }

    #[test]
    fn sign_rejects_invalid_base64_secret() {
        let result = sign("not-valid-base64!!!", URL_PATH, 123, "nonce=123");
        assert!(result.is_err());
    }
}
