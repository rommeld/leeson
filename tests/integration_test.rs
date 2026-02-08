//! Real API integration tests for Kraken WebSocket V2.
//!
//! These tests connect to the live Kraken WebSocket API and require network access.
//! Run with: `cargo test --features integration-tests`

#![cfg(feature = "integration-tests")]

mod common;

use futures_util::StreamExt;
use leeson::models::Channel;
use leeson::websocket::{connect, ping, subscribe, unsubscribe};

use common::KRAKEN_WS_URL;

#[tokio::test]
async fn test_connect_to_kraken_websocket() {
    let result = connect(KRAKEN_WS_URL, common::test_tls_config()).await;
    assert!(result.is_ok(), "Failed to connect to Kraken WebSocket");
}

#[tokio::test]
async fn test_ping_pong() {
    let (mut write, mut read) = connect(KRAKEN_WS_URL, common::test_tls_config())
        .await
        .expect("Failed to connect");

    // Send ping
    ping(&mut write).await.expect("Failed to send ping");

    // Wait for pong response (with timeout)
    let timeout = tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
        while let Some(msg) = read.next().await {
            if let Ok(tungstenite::Message::Text(text)) = msg {
                if text.contains("\"method\":\"pong\"") {
                    return true;
                }
            }
        }
        false
    });

    let received_pong = timeout.await.expect("Timeout waiting for pong");
    assert!(received_pong, "Did not receive pong response");
}

#[tokio::test]
async fn test_subscribe_and_receive_ticker() {
    let (mut write, mut read) = connect(KRAKEN_WS_URL, common::test_tls_config())
        .await
        .expect("Failed to connect");

    let symbols = vec!["BTC/USD".to_string()];

    // Subscribe to ticker
    subscribe(&mut write, &Channel::Ticker, &symbols, None)
        .await
        .expect("Failed to subscribe to ticker");

    // Wait for at least one ticker message (with timeout)
    let timeout = tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
        while let Some(msg) = read.next().await {
            if let Ok(tungstenite::Message::Text(text)) = msg {
                if text.contains("\"channel\":\"ticker\"") {
                    return true;
                }
            }
        }
        false
    });

    let received_ticker = timeout.await.expect("Timeout waiting for ticker");
    assert!(received_ticker, "Did not receive ticker message");

    // Clean up: unsubscribe
    unsubscribe(&mut write, &Channel::Ticker, &symbols, None)
        .await
        .expect("Failed to unsubscribe from ticker");
}

#[tokio::test]
async fn test_subscribe_and_receive_book() {
    let (mut write, mut read) = connect(KRAKEN_WS_URL, common::test_tls_config())
        .await
        .expect("Failed to connect");

    let symbols = vec!["BTC/USD".to_string()];

    // Subscribe to book
    subscribe(&mut write, &Channel::Book, &symbols, None)
        .await
        .expect("Failed to subscribe to book");

    // Wait for at least one book message (snapshot or update)
    let timeout = tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
        while let Some(msg) = read.next().await {
            if let Ok(tungstenite::Message::Text(text)) = msg {
                if text.contains("\"channel\":\"book\"") {
                    return true;
                }
            }
        }
        false
    });

    let received_book = timeout.await.expect("Timeout waiting for book");
    assert!(received_book, "Did not receive book message");

    // Clean up: unsubscribe
    unsubscribe(&mut write, &Channel::Book, &symbols, None)
        .await
        .expect("Failed to unsubscribe from book");
}
