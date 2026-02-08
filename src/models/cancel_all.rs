//! Cancel all orders RPC models.
//!
//! Provides types for cancelling all open orders via the Kraken WebSocket V2 API.
//! This cancels all orders including untriggered orders and orders resting in the book.

use serde::{Deserialize, Serialize};

/// Parameters for the cancel_all request.
#[derive(Debug, Clone, Serialize)]
pub struct CancelAllParams {
    pub token: super::RedactedToken,
}

/// The cancel_all request message.
#[derive(Debug, Clone, Serialize)]
pub struct CancelAllRequest {
    method: String,
    params: CancelAllParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    req_id: Option<u64>,
}

impl CancelAllRequest {
    /// Creates a new cancel_all request.
    #[must_use]
    pub fn new(token: &str, req_id: Option<u64>) -> Self {
        Self {
            method: "cancel_all".to_string(),
            params: CancelAllParams {
                token: super::RedactedToken::new(token),
            },
            req_id,
        }
    }

    /// Returns the request ID if set.
    #[must_use]
    pub fn req_id(&self) -> Option<u64> {
        self.req_id
    }
}

/// Successful cancel_all result.
#[derive(Debug, Clone, Deserialize)]
pub struct CancelAllResult {
    /// Number of orders cancelled.
    pub count: u64,
    /// Advisory messages about deprecated fields.
    #[serde(default)]
    pub warnings: Option<Vec<String>>,
}

/// Response to a cancel_all request.
#[derive(Debug, Clone, Deserialize)]
pub struct CancelAllResponse {
    pub method: String,
    pub success: bool,
    #[serde(default)]
    pub result: Option<CancelAllResult>,
    #[serde(default)]
    pub error: Option<String>,
    pub time_in: String,
    pub time_out: String,
    #[serde(default)]
    pub req_id: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_cancel_all_request() {
        let request = CancelAllRequest::new("test_token", Some(1234567890));

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "cancel_all");
        assert_eq!(value["req_id"], 1234567890);
        assert_eq!(value["params"]["token"], "test_token");
    }

    #[test]
    fn serialize_cancel_all_request_without_req_id() {
        let request = CancelAllRequest::new("test_token", None);

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "cancel_all");
        assert!(value.get("req_id").is_none());
    }

    #[test]
    fn deserialize_success_response() {
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

        let response: CancelAllResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.req_id, Some(1234567890));
        assert_eq!(response.result.as_ref().unwrap().count, 5);
    }

    #[test]
    fn deserialize_success_response_zero_cancelled() {
        let json = r#"{
            "method": "cancel_all",
            "result": {
                "count": 0
            },
            "success": true,
            "time_in": "2023-09-26T13:09:48.463201Z",
            "time_out": "2023-09-26T13:09:48.471419Z"
        }"#;

        let response: CancelAllResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.result.as_ref().unwrap().count, 0);
    }

    #[test]
    fn deserialize_error_response() {
        let json = r#"{
            "method": "cancel_all",
            "success": false,
            "error": "EGeneral:Permission denied",
            "time_in": "2023-09-26T13:09:48.463201Z",
            "time_out": "2023-09-26T13:09:48.471419Z"
        }"#;

        let response: CancelAllResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(
            response.error,
            Some("EGeneral:Permission denied".to_string())
        );
        assert!(response.result.is_none());
    }

    #[test]
    fn deserialize_response_with_warnings() {
        let json = r#"{
            "method": "cancel_all",
            "result": {
                "count": 2,
                "warnings": ["Deprecated field used"]
            },
            "success": true,
            "time_in": "2023-09-26T13:09:48.463201Z",
            "time_out": "2023-09-26T13:09:48.471419Z"
        }"#;

        let response: CancelAllResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.result.as_ref().unwrap().count, 2);
        assert_eq!(
            response.result.as_ref().unwrap().warnings,
            Some(vec!["Deprecated field used".to_string()])
        );
    }
}
