//! Serialization tests for WebSocket request types and Channel enum.

use leeson::models::{
    BatchAddBuilder, BatchAddResponse, BatchCancelBuilder, BatchCancelRequest, BatchCancelResponse,
    BatchOrderEntry, CancelAfterRequest, CancelAfterResponse, CancelAllRequest, CancelAllResponse,
    CancelOrderBuilder, CancelOrderResponse, Channel, ExecutionsSubscribeRequest,
    ExecutionsUnsubscribeRequest, OrderSide, PingRequest, SubscribeRequest, UnsubscribeRequest,
};
use rust_decimal_macros::dec;

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

    let json = serde_json::to_string(&request).expect("Failed to serialize cancel_order request");
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

    let json = serde_json::to_string(&request).expect("Failed to serialize cancel_order request");
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

    let json = serde_json::to_string(&request).expect("Failed to serialize cancel_order request");
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

#[test]
fn test_cancel_all_request_serializes() {
    let request = CancelAllRequest::new("ws-token-123", Some(1234567890));

    let json = serde_json::to_string(&request).expect("Failed to serialize cancel_all request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "cancel_all");
    assert_eq!(value["req_id"], 1234567890);
    assert_eq!(value["params"]["token"], "ws-token-123");
}

#[test]
fn test_cancel_all_request_without_req_id_serializes() {
    let request = CancelAllRequest::new("ws-token-123", None);

    let json = serde_json::to_string(&request).expect("Failed to serialize cancel_all request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "cancel_all");
    assert!(value.get("req_id").is_none());
}

#[test]
fn test_cancel_all_success_response_deserializes() {
    let json = r#"{
        "method": "cancel_all",
        "req_id": 1234567890,
        "result": {
            "count": 5
        },
        "success": true,
        "time_in": "2023-09-26T13:09:48.463201Z",
        "time_out": "2023-09-26T13:09:48.471419Z"
    }"#;

    let response: CancelAllResponse =
        serde_json::from_str(json).expect("Failed to deserialize cancel_all response");

    assert_eq!(response.method, "cancel_all");
    assert!(response.success);
    assert_eq!(response.req_id, Some(1234567890));
    assert!(response.result.is_some());
    assert_eq!(response.result.unwrap().count, 5);
}

#[test]
fn test_cancel_all_error_response_deserializes() {
    let json = r#"{
        "method": "cancel_all",
        "success": false,
        "error": "EGeneral:Permission denied",
        "time_in": "2023-09-26T13:09:48.463201Z",
        "time_out": "2023-09-26T13:09:48.471419Z"
    }"#;

    let response: CancelAllResponse =
        serde_json::from_str(json).expect("Failed to deserialize cancel_all error response");

    assert_eq!(response.method, "cancel_all");
    assert!(!response.success);
    assert_eq!(
        response.error,
        Some("EGeneral:Permission denied".to_string())
    );
    assert!(response.result.is_none());
}

// Dead Man's Switch (cancel_after) tests

#[test]
fn test_cancel_after_request_serializes() {
    let request = CancelAfterRequest::new(60, "ws-token-123", Some(1234567890));

    let json = serde_json::to_string(&request).expect("Failed to serialize cancel_after request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "cancel_all_orders_after");
    assert_eq!(value["req_id"], 1234567890);
    assert_eq!(value["params"]["timeout"], 60);
    assert_eq!(value["params"]["token"], "ws-token-123");
}

#[test]
fn test_cancel_after_disable_request_serializes() {
    let request = CancelAfterRequest::disable("ws-token-123", None);

    let json = serde_json::to_string(&request).expect("Failed to serialize cancel_after request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "cancel_all_orders_after");
    assert_eq!(value["params"]["timeout"], 0);
    assert!(value.get("req_id").is_none());
}

#[test]
fn test_cancel_after_success_response_deserializes() {
    let json = r#"{
        "method": "cancel_all_orders_after",
        "req_id": 1234567890,
        "result": {
            "currentTime": "2023-09-21T15:49:29Z",
            "triggerTime": "2023-09-21T15:51:09Z"
        },
        "success": true,
        "time_in": "2023-09-21T15:49:28.627900Z",
        "time_out": "2023-09-21T15:49:28.649057Z"
    }"#;

    let response: CancelAfterResponse =
        serde_json::from_str(json).expect("Failed to deserialize cancel_after response");

    assert_eq!(response.method, "cancel_all_orders_after");
    assert!(response.success);
    assert_eq!(response.req_id, Some(1234567890));
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert_eq!(result.current_time, "2023-09-21T15:49:29Z");
    assert_eq!(result.trigger_time, "2023-09-21T15:51:09Z");
}

#[test]
fn test_cancel_after_error_response_deserializes() {
    let json = r#"{
        "method": "cancel_all_orders_after",
        "success": false,
        "error": "EGeneral:Invalid arguments",
        "time_in": "2023-09-21T15:49:28.627900Z",
        "time_out": "2023-09-21T15:49:28.649057Z"
    }"#;

    let response: CancelAfterResponse =
        serde_json::from_str(json).expect("Failed to deserialize cancel_after error response");

    assert_eq!(response.method, "cancel_all_orders_after");
    assert!(!response.success);
    assert_eq!(
        response.error,
        Some("EGeneral:Invalid arguments".to_string())
    );
    assert!(response.result.is_none());
}

// Batch add tests

#[test]
fn test_batch_add_request_serializes() {
    let request = BatchAddBuilder::new("BTC/USD")
        .add_order(
            BatchOrderEntry::limit(OrderSide::Buy, dec!(0.1), dec!(50000)).with_order_userref(1),
        )
        .add_order(
            BatchOrderEntry::limit(OrderSide::Sell, dec!(0.2), dec!(55000)).with_order_userref(2),
        )
        .with_req_id(123456789)
        .build("ws-token-123")
        .expect("Failed to build batch_add request");

    let json = serde_json::to_string(&request).expect("Failed to serialize batch_add request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "batch_add");
    assert_eq!(value["req_id"], 123456789);
    assert_eq!(value["params"]["symbol"], "BTC/USD");
    assert_eq!(value["params"]["token"], "ws-token-123");
    assert_eq!(value["params"]["orders"].as_array().unwrap().len(), 2);
    assert_eq!(value["params"]["orders"][0]["side"], "buy");
    assert_eq!(value["params"]["orders"][0]["order_type"], "limit");
    assert_eq!(value["params"]["orders"][1]["side"], "sell");
}

#[test]
fn test_batch_add_with_validate_serializes() {
    let request = BatchAddBuilder::new("ETH/USD")
        .add_order(BatchOrderEntry::market(OrderSide::Buy, dec!(1)))
        .add_order(BatchOrderEntry::market(OrderSide::Sell, dec!(1)))
        .with_validate(true)
        .build("token")
        .expect("Failed to build batch_add request");

    let json = serde_json::to_string(&request).expect("Failed to serialize batch_add request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["params"]["validate"], true);
}

#[test]
fn test_batch_add_success_response_deserializes() {
    let json = r#"{
        "method": "batch_add",
        "req_id": 1234567890,
        "result": [
            {
                "order_id": "ORDERX-IDXXX-XXXXX1",
                "cl_ord_id": "my-order-1",
                "order_userref": 1
            },
            {
                "order_id": "ORDERX-IDXXX-XXXXX2",
                "order_userref": 2
            }
        ],
        "success": true,
        "time_in": "2022-06-13T08:09:10.123456Z",
        "time_out": "2022-06-13T08:09:10.789012Z"
    }"#;

    let response: BatchAddResponse =
        serde_json::from_str(json).expect("Failed to deserialize batch_add response");

    assert_eq!(response.method, "batch_add");
    assert!(response.success);
    assert_eq!(response.req_id, Some(1234567890));
    assert!(response.result.is_some());

    let results = response.result.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].order_id, "ORDERX-IDXXX-XXXXX1");
    assert_eq!(results[0].cl_ord_id, Some("my-order-1".to_string()));
    assert_eq!(results[1].order_id, "ORDERX-IDXXX-XXXXX2");
}

#[test]
fn test_batch_add_error_response_deserializes() {
    let json = r#"{
        "method": "batch_add",
        "success": false,
        "error": "EOrder:Invalid order",
        "time_in": "2022-06-13T08:09:10.123456Z",
        "time_out": "2022-06-13T08:09:10.789012Z"
    }"#;

    let response: BatchAddResponse =
        serde_json::from_str(json).expect("Failed to deserialize batch_add error response");

    assert_eq!(response.method, "batch_add");
    assert!(!response.success);
    assert_eq!(response.error, Some("EOrder:Invalid order".to_string()));
    assert!(response.result.is_none());
}

// Batch cancel tests

#[test]
fn test_batch_cancel_request_serializes() {
    let request = BatchCancelRequest::new(
        vec![
            "1".to_string(),
            "2".to_string(),
            "ORDERX-IDXXX-XXXXX3".to_string(),
        ],
        "ws-token-123",
        Some(1234567890),
    );

    let json = serde_json::to_string(&request).expect("Failed to serialize batch_cancel request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "batch_cancel");
    assert_eq!(value["req_id"], 1234567890);
    assert_eq!(value["params"]["token"], "ws-token-123");
    assert_eq!(value["params"]["orders"].as_array().unwrap().len(), 3);
    assert_eq!(value["params"]["orders"][0], "1");
    assert_eq!(value["params"]["orders"][1], "2");
    assert_eq!(value["params"]["orders"][2], "ORDERX-IDXXX-XXXXX3");
}

#[test]
fn test_batch_cancel_builder_serializes() {
    let request = BatchCancelBuilder::with_orders(vec!["1".to_string(), "2".to_string()])
        .with_req_id(42)
        .build("ws-token-456")
        .expect("Failed to build batch_cancel request");

    let json = serde_json::to_string(&request).expect("Failed to serialize batch_cancel request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["method"], "batch_cancel");
    assert_eq!(value["req_id"], 42);
    assert_eq!(value["params"]["orders"].as_array().unwrap().len(), 2);
}

#[test]
fn test_batch_cancel_with_cl_ord_id_serializes() {
    let request = BatchCancelRequest::new(
        vec!["1".to_string(), "2".to_string()],
        "ws-token-123",
        Some(42),
    )
    .with_cl_ord_id(vec!["client-1".to_string(), "client-2".to_string()]);

    let json = serde_json::to_string(&request).expect("Failed to serialize batch_cancel request");
    let value: serde_json::Value =
        serde_json::from_str(&json).expect("Failed to parse serialized JSON");

    assert_eq!(value["params"]["cl_ord_id"][0], "client-1");
    assert_eq!(value["params"]["cl_ord_id"][1], "client-2");
}

#[test]
fn test_batch_cancel_success_response_deserializes() {
    let json = r#"{
        "method": "batch_cancel",
        "req_id": 1234567890,
        "result": {
            "count": 3
        },
        "success": true,
        "time_in": "2022-06-13T08:09:10.123456Z",
        "time_out": "2022-06-13T08:09:10.7890123"
    }"#;

    let response: BatchCancelResponse =
        serde_json::from_str(json).expect("Failed to deserialize batch_cancel response");

    assert_eq!(response.method, "batch_cancel");
    assert!(response.success);
    assert_eq!(response.req_id, Some(1234567890));
    assert!(response.result.is_some());
    assert_eq!(response.result.unwrap().count, 3);
}

#[test]
fn test_batch_cancel_error_response_deserializes() {
    let json = r#"{
        "method": "batch_cancel",
        "success": false,
        "error": "EGeneral:Permission denied",
        "time_in": "2022-06-13T08:09:10.123456Z",
        "time_out": "2022-06-13T08:09:10.789012Z"
    }"#;

    let response: BatchCancelResponse =
        serde_json::from_str(json).expect("Failed to deserialize batch_cancel error response");

    assert_eq!(response.method, "batch_cancel");
    assert!(!response.success);
    assert_eq!(
        response.error,
        Some("EGeneral:Permission denied".to_string())
    );
    assert!(response.result.is_none());
}
