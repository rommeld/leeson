//! Cancel order RPC models.
//!
//! Provides types for cancelling orders via the Kraken WebSocket V2 API.
//! Like `add_order`, this is an RPC-style one-shot request/response command.

use serde::{Deserialize, Serialize};

/// Parameters for the cancel_order request.
#[derive(Debug, Clone, Serialize)]
pub struct CancelOrderParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cl_ord_id: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_userref: Option<Vec<i64>>,
    pub token: super::RedactedToken,
}

/// The cancel_order request message.
#[derive(Debug, Clone, Serialize)]
pub struct CancelOrderRequest {
    method: String,
    params: CancelOrderParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    req_id: Option<u64>,
}

impl CancelOrderRequest {
    /// Creates a new cancel_order request from params and optional request ID.
    #[must_use]
    pub fn new(params: CancelOrderParams, req_id: Option<u64>) -> Self {
        Self {
            method: "cancel_order".to_string(),
            params,
            req_id,
        }
    }

    /// Returns the request ID if set.
    #[must_use]
    pub fn req_id(&self) -> Option<u64> {
        self.req_id
    }
}

/// Successful order cancellation result.
#[derive(Debug, Clone, Deserialize)]
pub struct CancelOrderResult {
    pub order_id: String,
    #[serde(default)]
    pub cl_ord_id: Option<String>,
}

/// Response to a cancel_order request.
#[derive(Debug, Clone, Deserialize)]
pub struct CancelOrderResponse {
    pub method: String,
    pub success: bool,
    #[serde(default)]
    pub result: Option<CancelOrderResult>,
    #[serde(default)]
    pub error: Option<String>,
    pub time_in: String,
    pub time_out: String,
    #[serde(default)]
    pub req_id: Option<u64>,
    #[serde(default)]
    pub warnings: Option<Vec<String>>,
}

/// Builder for constructing cancel_order requests.
#[derive(Debug, Clone, Default)]
pub struct CancelOrderBuilder {
    order_id: Option<Vec<String>>,
    cl_ord_id: Option<Vec<String>>,
    order_userref: Option<Vec<i64>>,
    req_id: Option<u64>,
}

impl CancelOrderBuilder {
    /// Creates a new empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a builder to cancel orders by Kraken order ID.
    #[must_use]
    pub fn by_order_id(order_ids: Vec<String>) -> Self {
        Self {
            order_id: Some(order_ids),
            ..Default::default()
        }
    }

    /// Creates a builder to cancel a single order by Kraken order ID.
    #[must_use]
    pub fn by_single_order_id(order_id: &str) -> Self {
        Self::by_order_id(vec![order_id.to_string()])
    }

    /// Creates a builder to cancel orders by client order ID.
    #[must_use]
    pub fn by_cl_ord_id(cl_ord_ids: Vec<String>) -> Self {
        Self {
            cl_ord_id: Some(cl_ord_ids),
            ..Default::default()
        }
    }

    /// Creates a builder to cancel a single order by client order ID.
    #[must_use]
    pub fn by_single_cl_ord_id(cl_ord_id: &str) -> Self {
        Self::by_cl_ord_id(vec![cl_ord_id.to_string()])
    }

    /// Creates a builder to cancel orders by user reference.
    #[must_use]
    pub fn by_order_userref(userrefs: Vec<i64>) -> Self {
        Self {
            order_userref: Some(userrefs),
            ..Default::default()
        }
    }

    /// Creates a builder to cancel a single order by user reference.
    #[must_use]
    pub fn by_single_order_userref(userref: i64) -> Self {
        Self::by_order_userref(vec![userref])
    }

    /// Adds Kraken order IDs to cancel.
    #[must_use]
    pub fn with_order_id(mut self, order_ids: Vec<String>) -> Self {
        self.order_id = Some(order_ids);
        self
    }

    /// Adds client order IDs to cancel.
    #[must_use]
    pub fn with_cl_ord_id(mut self, cl_ord_ids: Vec<String>) -> Self {
        self.cl_ord_id = Some(cl_ord_ids);
        self
    }

    /// Adds user references to cancel.
    #[must_use]
    pub fn with_order_userref(mut self, userrefs: Vec<i64>) -> Self {
        self.order_userref = Some(userrefs);
        self
    }

    /// Sets the request ID for correlation.
    #[must_use]
    pub fn with_req_id(mut self, req_id: u64) -> Self {
        self.req_id = Some(req_id);
        self
    }

    /// Validates and builds the cancel order parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if no order identifiers are provided.
    pub fn build(self, token: &str) -> Result<CancelOrderParams, CancelOrderError> {
        self.validate()?;

        Ok(CancelOrderParams {
            order_id: self.order_id,
            cl_ord_id: self.cl_ord_id,
            order_userref: self.order_userref,
            token: super::RedactedToken::new(token),
        })
    }

    /// Validates and builds the full request.
    ///
    /// # Errors
    ///
    /// Returns an error if no order identifiers are provided.
    pub fn build_request(self, token: &str) -> Result<CancelOrderRequest, CancelOrderError> {
        let req_id = self.req_id;
        let params = self.build(token)?;
        Ok(CancelOrderRequest::new(params, req_id))
    }

    fn validate(&self) -> Result<(), CancelOrderError> {
        let has_order_id = self.order_id.as_ref().is_some_and(|v| !v.is_empty());
        let has_cl_ord_id = self.cl_ord_id.as_ref().is_some_and(|v| !v.is_empty());
        let has_userref = self.order_userref.as_ref().is_some_and(|v| !v.is_empty());

        if !has_order_id && !has_cl_ord_id && !has_userref {
            return Err(CancelOrderError::NoOrderIdentifier);
        }

        Ok(())
    }
}

/// Errors that can occur when building a cancel_order request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelOrderError {
    /// At least one order identifier must be provided.
    NoOrderIdentifier,
}

impl std::fmt::Display for CancelOrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOrderIdentifier => write!(
                f,
                "at least one of order_id, cl_ord_id, or order_userref must be provided"
            ),
        }
    }
}

impl std::error::Error for CancelOrderError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_cancel_by_order_id() {
        let request = CancelOrderBuilder::by_order_id(vec![
            "OM5CRX-N2HAL-GFGWE9".to_string(),
            "OLUMT4-UTEGU-ZYM7E9".to_string(),
        ])
        .with_req_id(123456789)
        .build_request("test_token")
        .unwrap();

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "cancel_order");
        assert_eq!(value["req_id"], 123456789);
        assert_eq!(value["params"]["token"], "test_token");
        assert_eq!(
            value["params"]["order_id"],
            serde_json::json!(["OM5CRX-N2HAL-GFGWE9", "OLUMT4-UTEGU-ZYM7E9"])
        );
        assert!(value["params"].get("cl_ord_id").is_none());
    }

    #[test]
    fn serialize_cancel_by_single_order_id() {
        let request = CancelOrderBuilder::by_single_order_id("OM5CRX-N2HAL-GFGWE9")
            .build_request("test_token")
            .unwrap();

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            value["params"]["order_id"],
            serde_json::json!(["OM5CRX-N2HAL-GFGWE9"])
        );
    }

    #[test]
    fn serialize_cancel_by_cl_ord_id() {
        let request = CancelOrderBuilder::by_cl_ord_id(vec!["my-order-1".to_string()])
            .build_request("test_token")
            .unwrap();

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            value["params"]["cl_ord_id"],
            serde_json::json!(["my-order-1"])
        );
        assert!(value["params"].get("order_id").is_none());
    }

    #[test]
    fn serialize_cancel_by_order_userref() {
        let request = CancelOrderBuilder::by_order_userref(vec![12345, 67890])
            .build_request("test_token")
            .unwrap();

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            value["params"]["order_userref"],
            serde_json::json!([12345, 67890])
        );
    }

    #[test]
    fn validate_requires_identifier() {
        let result = CancelOrderBuilder::new().build("token");

        assert!(matches!(result, Err(CancelOrderError::NoOrderIdentifier)));
    }

    #[test]
    fn validate_empty_arrays_rejected() {
        let result = CancelOrderBuilder::new()
            .with_order_id(vec![])
            .build("token");

        assert!(matches!(result, Err(CancelOrderError::NoOrderIdentifier)));
    }

    #[test]
    fn deserialize_success_response() {
        let json = r#"{
            "method": "cancel_order",
            "success": true,
            "result": {
                "order_id": "OLUMT4-UTEGU-ZYM7E9",
                "cl_ord_id": "my-order-1"
            },
            "time_in": "2023-09-21T14:36:57.428972Z",
            "time_out": "2023-09-21T14:36:57.437952Z",
            "req_id": 123456789
        }"#;

        let response: CancelOrderResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(
            response.result.as_ref().unwrap().order_id,
            "OLUMT4-UTEGU-ZYM7E9"
        );
        assert_eq!(
            response.result.as_ref().unwrap().cl_ord_id,
            Some("my-order-1".to_string())
        );
        assert_eq!(response.req_id, Some(123456789));
    }

    #[test]
    fn deserialize_error_response() {
        let json = r#"{
            "method": "cancel_order",
            "success": false,
            "error": "EOrder:Unknown order",
            "time_in": "2023-09-21T14:36:57.428972Z",
            "time_out": "2023-09-21T14:36:57.437952Z"
        }"#;

        let response: CancelOrderResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(response.error, Some("EOrder:Unknown order".to_string()));
        assert!(response.result.is_none());
    }

    #[test]
    fn deserialize_response_with_warnings() {
        let json = r#"{
            "method": "cancel_order",
            "success": true,
            "result": {"order_id": "OLUMT4-UTEGU-ZYM7E9"},
            "time_in": "2023-09-21T14:36:57.428972Z",
            "time_out": "2023-09-21T14:36:57.437952Z",
            "warnings": ["Deprecated field used"]
        }"#;

        let response: CancelOrderResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(
            response.warnings,
            Some(vec!["Deprecated field used".to_string()])
        );
    }
}
