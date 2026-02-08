//! Serialization tests for WebSocket request types and Channel enum.

use leeson::models::{
    CancelOrderBuilder, CancelOrderResponse, Channel, ExecutionsSubscribeRequest,
    ExecutionsUnsubscribeRequest, PingRequest, SubscribeRequest, UnsubscribeRequest,
};

#[test]
fn test_channel_as_str_returns_correct_wire_names() {
    assert_eq!(Channel::Book.as_str(), "book");
    assert_eq!(Channel::Ticker.as_str(), "ticker");
    assert_eq!(Channel::Orders.as_str(), "level3");
    assert_eq!(Channel::Candles.as_str(), "ohlc");
    assert_eq!(Channel::Trades.as_str(), "trade");
    assert_eq!(Channel::Instruments.as_str(), "instrument");
    assert_eq!(Channel::Executions.as_str(), "executions");
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

#[test]
fn test_executions_subscribe_request_serializes() {
    let request = ExecutionsSubscribeRequest::new("ws-token-456", true, false);

    let json =
        serde_json::to_string(&request).expect("Failed to serialize executions subscribe request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "subscribe");
    assert_eq!(value["params"]["channel"], "executions");
    assert_eq!(value["params"]["token"], "ws-token-456");
    assert_eq!(value["params"]["snap_orders"], true);
    assert_eq!(value["params"]["snap_trades"], false);
}

#[test]
fn test_executions_unsubscribe_request_serializes() {
    let request = ExecutionsUnsubscribeRequest::new("ws-token-456");

    let json = serde_json::to_string(&request)
        .expect("Failed to serialize executions unsubscribe request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "unsubscribe");
    assert_eq!(value["params"]["channel"], "executions");
    assert_eq!(value["params"]["token"], "ws-token-456");
}

#[test]
fn test_cancel_order_by_order_id_serializes() {
    let request = CancelOrderBuilder::by_order_id(vec![
        "OM5CRX-N2HAL-GFGWE9".to_string(),
        "OLUMT4-UTEGU-ZYM7E9".to_string(),
    ])
    .with_req_id(123456789)
    .build_request("ws-token-789")
    .expect("Failed to build cancel_order request");

    let json =
        serde_json::to_string(&request).expect("Failed to serialize cancel_order request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "cancel_order");
    assert_eq!(value["req_id"], 123456789);
    assert_eq!(value["params"]["token"], "ws-token-789");
    assert_eq!(value["params"]["order_id"][0], "OM5CRX-N2HAL-GFGWE9");
    assert_eq!(value["params"]["order_id"][1], "OLUMT4-UTEGU-ZYM7E9");
    assert!(value["params"].get("cl_ord_id").is_none());
    assert!(value["params"].get("order_userref").is_none());
}

#[test]
fn test_cancel_order_by_cl_ord_id_serializes() {
    let request = CancelOrderBuilder::by_cl_ord_id(vec!["my-order-1".to_string()])
        .build_request("ws-token-789")
        .expect("Failed to build cancel_order request");

    let json =
        serde_json::to_string(&request).expect("Failed to serialize cancel_order request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "cancel_order");
    assert_eq!(value["params"]["cl_ord_id"][0], "my-order-1");
    assert!(value["params"].get("order_id").is_none());
    assert!(value.get("req_id").is_none());
}

#[test]
fn test_cancel_order_by_order_userref_serializes() {
    let request = CancelOrderBuilder::by_order_userref(vec![12345, 67890])
        .build_request("ws-token-789")
        .expect("Failed to build cancel_order request");

    let json =
        serde_json::to_string(&request).expect("Failed to serialize cancel_order request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "cancel_order");
    assert_eq!(value["params"]["order_userref"][0], 12345);
    assert_eq!(value["params"]["order_userref"][1], 67890);
}

#[test]
fn test_cancel_order_success_response_deserializes() {
    let json = r#"{
        "method": "cancel_order",
        "req_id": 123456789,
        "result": {
            "order_id": "OLUMT4-UTEGU-ZYM7E9",
            "cl_ord_id": "my-order-1"
        },
        "success": true,
        "time_in": "2023-09-21T14:36:57.428972Z",
        "time_out": "2023-09-21T14:36:57.437952Z"
    }"#;

    let response: CancelOrderResponse =
        serde_json::from_str(json).expect("Failed to deserialize cancel_order response");

    assert_eq!(response.method, "cancel_order");
    assert!(response.success);
    assert_eq!(response.req_id, Some(123456789));
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert_eq!(result.order_id, "OLUMT4-UTEGU-ZYM7E9");
    assert_eq!(result.cl_ord_id, Some("my-order-1".to_string()));
}

#[test]
fn test_cancel_order_error_response_deserializes() {
    let json = r#"{
        "method": "cancel_order",
        "success": false,
        "error": "EOrder:Unknown order",
        "time_in": "2023-09-21T14:36:57.428972Z",
        "time_out": "2023-09-21T14:36:57.437952Z"
    }"#;

    let response: CancelOrderResponse =
        serde_json::from_str(json).expect("Failed to deserialize cancel_order error response");

    assert_eq!(response.method, "cancel_order");
    assert!(!response.success);
    assert_eq!(response.error, Some("EOrder:Unknown order".to_string()));
    assert!(response.result.is_none());
}

#[test]
fn test_cancel_order_response_with_warnings_deserializes() {
    let json = r#"{
        "method": "cancel_order",
        "success": true,
        "result": {"order_id": "OLUMT4-UTEGU-ZYM7E9"},
        "time_in": "2023-09-21T14:36:57.428972Z",
        "time_out": "2023-09-21T14:36:57.437952Z",
        "warnings": ["Deprecated field used", "Another warning"]
    }"#;

    let response: CancelOrderResponse =
        serde_json::from_str(json).expect("Failed to deserialize cancel_order response");

    assert!(response.success);
    assert!(response.warnings.is_some());

    let warnings = response.warnings.unwrap();
    assert_eq!(warnings.len(), 2);
    assert_eq!(warnings[0], "Deprecated field used");
    assert_eq!(warnings[1], "Another warning");
}
