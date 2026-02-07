//! Serialization tests for WebSocket request types and Channel enum.

use leeson::models::{Channel, PingRequest, SubscribeRequest, UnsubscribeRequest};

#[test]
fn test_channel_as_str_returns_correct_wire_names() {
    assert_eq!(Channel::Book.as_str(), "book");
    assert_eq!(Channel::Ticker.as_str(), "ticker");
    assert_eq!(Channel::Orders.as_str(), "level3");
    assert_eq!(Channel::Candles.as_str(), "ohlc");
    assert_eq!(Channel::Trades.as_str(), "trade");
    assert_eq!(Channel::Instruments.as_str(), "instrument");
    assert_eq!(Channel::Status.as_str(), "status");
    assert_eq!(Channel::Heartbeat.as_str(), "heartbeat");
}

#[test]
fn test_ping_request_serializes() {
    let request = PingRequest::new();

    let json = serde_json::to_string(&request).expect("Failed to serialize ping request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "ping");
}

#[test]
fn test_subscribe_request_serializes() {
    let symbols = vec!["BTC/USD".to_string(), "ETH/USD".to_string()];
    let request = SubscribeRequest::new(&Channel::Ticker, &symbols, None);

    let json = serde_json::to_string(&request).expect("Failed to serialize subscribe request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "subscribe");
    assert_eq!(value["params"]["channel"], "ticker");
    assert_eq!(value["params"]["symbol"][0], "BTC/USD");
    assert_eq!(value["params"]["symbol"][1], "ETH/USD");
    assert!(value["params"].get("token").is_none());
}

#[test]
fn test_unsubscribe_request_serializes() {
    let symbols = vec!["BTC/USD".to_string()];
    let request = UnsubscribeRequest::new(&Channel::Book, &symbols, None);

    let json = serde_json::to_string(&request).expect("Failed to serialize unsubscribe request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "unsubscribe");
    assert_eq!(value["params"]["channel"], "book");
    assert_eq!(value["params"]["symbol"][0], "BTC/USD");
}

#[test]
fn test_subscribe_request_with_channel_enum() {
    let channel = Channel::Ticker;
    let symbols = vec!["BTC/USD".to_string()];
    let request = SubscribeRequest::new(&channel, &symbols, None);

    let json = serde_json::to_string(&request).expect("Failed to serialize subscribe request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["params"]["channel"], "ticker");
}

#[test]
fn test_subscribe_request_with_token_serializes() {
    let symbols = vec!["BTC/USD".to_string()];
    let request = SubscribeRequest::new(
        &Channel::Orders,
        &symbols,
        Some("test-token-123".to_string()),
    );

    let json = serde_json::to_string(&request).expect("Failed to serialize subscribe request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "subscribe");
    assert_eq!(value["params"]["channel"], "level3");
    assert_eq!(value["params"]["symbol"][0], "BTC/USD");
    assert_eq!(value["params"]["token"], "test-token-123");
}
