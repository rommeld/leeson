//! Dead Man's Switch (cancel_all_orders_after) RPC models.
//!
//! Provides types for the "Dead Man's Switch" mechanism that protects against
//! network failures by automatically cancelling all orders if a countdown
//! timer expires without being refreshed.
//!
//! # Usage Pattern
//!
//! 1. Send a request with a timeout (e.g., 60 seconds) to start the countdown
//! 2. Periodically send requests (e.g., every 15-30 seconds) to reset the timer
//! 3. If the timer expires, all orders are automatically cancelled
//! 4. Send a request with timeout=0 to disable the feature
//!
//! # Example
//!
//! ```ignore
//! // Enable dead man's switch with 60 second timeout
//! let request = CancelAfterRequest::new(60, token, None);
//! cancel_after(&mut write, request).await?;
//!
//! // Disable the dead man's switch
//! let request = CancelAfterRequest::disable(token, None);
//! cancel_after(&mut write, request).await?;
//! ```

use serde::{Deserialize, Serialize};

/// Maximum timeout value in seconds (24 hours).
pub const MAX_TIMEOUT_SECONDS: u32 = 86400;

/// Parameters for the cancel_all_orders_after request.
#[derive(Debug, Clone, Serialize)]
pub struct CancelAfterParams {
    /// Duration in seconds until orders are cancelled. Set to 0 to disable.
    pub timeout: u32,
    pub token: String,
}

/// The cancel_all_orders_after request message.
#[derive(Debug, Clone, Serialize)]
pub struct CancelAfterRequest {
    method: String,
    params: CancelAfterParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    req_id: Option<u64>,
}

impl CancelAfterRequest {
    /// Creates a new cancel_after request with the specified timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Seconds until orders are cancelled (0 to disable, max 86400)
    /// * `token` - Authentication token
    /// * `req_id` - Optional request ID for correlation
    #[must_use]
    pub fn new(timeout: u32, token: &str, req_id: Option<u64>) -> Self {
        Self {
            method: "cancel_all_orders_after".to_string(),
            params: CancelAfterParams {
                timeout,
                token: token.to_string(),
            },
            req_id,
        }
    }

    /// Creates a request to disable the dead man's switch.
    #[must_use]
    pub fn disable(token: &str, req_id: Option<u64>) -> Self {
        Self::new(0, token, req_id)
    }

    /// Returns the request ID if set.
    #[must_use]
    pub fn req_id(&self) -> Option<u64> {
        self.req_id
    }

    /// Returns the timeout value in seconds.
    #[must_use]
    pub fn timeout(&self) -> u32 {
        self.params.timeout
    }
}

/// Successful cancel_after result.
#[derive(Debug, Clone, Deserialize)]
pub struct CancelAfterResult {
    /// Current engine time.
    #[serde(rename = "currentTime")]
    pub current_time: String,
    /// When orders will be cancelled if timer is not refreshed.
    #[serde(rename = "triggerTime")]
    pub trigger_time: String,
}

/// Response to a cancel_all_orders_after request.
#[derive(Debug, Clone, Deserialize)]
pub struct CancelAfterResponse {
    pub method: String,
    pub success: bool,
    #[serde(default)]
    pub result: Option<CancelAfterResult>,
    #[serde(default)]
    pub error: Option<String>,
    pub time_in: String,
    pub time_out: String,
    #[serde(default)]
    pub req_id: Option<u64>,
    #[serde(default)]
    pub warnings: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_cancel_after_request() {
        let request = CancelAfterRequest::new(60, "test_token", Some(1234567890));

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "cancel_all_orders_after");
        assert_eq!(value["req_id"], 1234567890);
        assert_eq!(value["params"]["timeout"], 60);
        assert_eq!(value["params"]["token"], "test_token");
    }

    #[test]
    fn serialize_cancel_after_disable_request() {
        let request = CancelAfterRequest::disable("test_token", None);

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "cancel_all_orders_after");
        assert_eq!(value["params"]["timeout"], 0);
        assert!(value.get("req_id").is_none());
    }

    #[test]
    fn serialize_cancel_after_request_without_req_id() {
        let request = CancelAfterRequest::new(100, "test_token", None);

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["params"]["timeout"], 100);
        assert!(value.get("req_id").is_none());
    }

    #[test]
    fn deserialize_success_response() {
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

        let response: CancelAfterResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.req_id, Some(1234567890));
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert_eq!(result.current_time, "2023-09-21T15:49:29Z");
        assert_eq!(result.trigger_time, "2023-09-21T15:51:09Z");
    }

    #[test]
    fn deserialize_error_response() {
        let json = r#"{
            "method": "cancel_all_orders_after",
            "success": false,
            "error": "EGeneral:Invalid arguments",
            "time_in": "2023-09-21T15:49:28.627900Z",
            "time_out": "2023-09-21T15:49:28.649057Z"
        }"#;

        let response: CancelAfterResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(response.error, Some("EGeneral:Invalid arguments".to_string()));
        assert!(response.result.is_none());
    }

    #[test]
    fn deserialize_response_with_warnings() {
        let json = r#"{
            "method": "cancel_all_orders_after",
            "result": {
                "currentTime": "2023-09-21T15:49:29Z",
                "triggerTime": "2023-09-21T15:51:09Z"
            },
            "success": true,
            "time_in": "2023-09-21T15:49:28.627900Z",
            "time_out": "2023-09-21T15:49:28.649057Z",
            "warnings": ["Deprecated field used"]
        }"#;

        let response: CancelAfterResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(
            response.warnings,
            Some(vec!["Deprecated field used".to_string()])
        );
    }

    #[test]
    fn request_accessors() {
        let request = CancelAfterRequest::new(60, "token", Some(123));
        assert_eq!(request.timeout(), 60);
        assert_eq!(request.req_id(), Some(123));

        let disable_request = CancelAfterRequest::disable("token", None);
        assert_eq!(disable_request.timeout(), 0);
        assert_eq!(disable_request.req_id(), None);
    }
}
