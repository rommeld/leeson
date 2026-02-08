//! Batch cancel orders RPC models.
//!
//! Provides types for cancelling multiple orders at once via the Kraken WebSocket V2 API.
//!
//! # Batch Rules
//!
//! - Minimum 2 orders, maximum 50 orders per batch
//! - Orders can be identified by `order_id` or `order_userref`

use serde::{Deserialize, Serialize};

/// Minimum number of orders in a batch cancel.
pub const MIN_BATCH_CANCEL_SIZE: usize = 2;

/// Maximum number of orders in a batch cancel.
pub const MAX_BATCH_CANCEL_SIZE: usize = 50;

/// Parameters for the batch_cancel request.
#[derive(Debug, Clone, Serialize)]
pub struct BatchCancelParams {
    pub orders: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cl_ord_id: Option<Vec<String>>,
    pub token: super::RedactedToken,
}

/// The batch_cancel request message.
#[derive(Debug, Clone, Serialize)]
pub struct BatchCancelRequest {
    method: String,
    params: BatchCancelParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    req_id: Option<u64>,
}

impl BatchCancelRequest {
    /// Creates a new batch_cancel request.
    ///
    /// # Arguments
    ///
    /// * `orders` - Order identifiers (order_id or order_userref)
    /// * `token` - Authentication token
    /// * `req_id` - Optional request ID for correlation
    #[must_use]
    pub fn new(orders: Vec<String>, token: &str, req_id: Option<u64>) -> Self {
        Self {
            method: "batch_cancel".to_string(),
            params: BatchCancelParams {
                orders,
                cl_ord_id: None,
                token: super::RedactedToken::new(token),
            },
            req_id,
        }
    }

    /// Sets client order identifiers.
    #[must_use]
    pub fn with_cl_ord_id(mut self, cl_ord_id: Vec<String>) -> Self {
        self.params.cl_ord_id = Some(cl_ord_id);
        self
    }

    /// Returns the request ID if set.
    #[must_use]
    pub fn req_id(&self) -> Option<u64> {
        self.req_id
    }

    /// Returns the number of orders in the batch.
    #[must_use]
    pub fn order_count(&self) -> usize {
        self.params.orders.len()
    }
}

/// Successful batch_cancel result.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchCancelResult {
    /// Number of orders cancelled.
    pub count: u64,
}

/// Response to a batch_cancel request.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchCancelResponse {
    pub method: String,
    pub success: bool,
    #[serde(default)]
    pub result: Option<BatchCancelResult>,
    #[serde(default)]
    pub error: Option<String>,
    pub time_in: String,
    pub time_out: String,
    #[serde(default)]
    pub req_id: Option<u64>,
}

/// Builder for constructing batch_cancel requests with validation.
#[derive(Debug, Clone)]
pub struct BatchCancelBuilder {
    orders: Vec<String>,
    cl_ord_id: Option<Vec<String>>,
    req_id: Option<u64>,
}

impl BatchCancelBuilder {
    /// Creates a new builder for a batch cancel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            orders: Vec::new(),
            cl_ord_id: None,
            req_id: None,
        }
    }

    /// Creates a builder with the given order identifiers.
    #[must_use]
    pub fn with_orders(orders: Vec<String>) -> Self {
        Self {
            orders,
            cl_ord_id: None,
            req_id: None,
        }
    }

    /// Adds an order identifier to cancel.
    #[must_use]
    pub fn add_order(mut self, order: &str) -> Self {
        self.orders.push(order.to_string());
        self
    }

    /// Adds multiple order identifiers to cancel.
    #[must_use]
    pub fn add_orders(mut self, orders: Vec<String>) -> Self {
        self.orders.extend(orders);
        self
    }

    /// Sets client order identifiers.
    #[must_use]
    pub fn with_cl_ord_id(mut self, cl_ord_id: Vec<String>) -> Self {
        self.cl_ord_id = Some(cl_ord_id);
        self
    }

    /// Sets the request ID for correlation.
    #[must_use]
    pub fn with_req_id(mut self, req_id: u64) -> Self {
        self.req_id = Some(req_id);
        self
    }

    /// Validates and builds the batch cancel request.
    ///
    /// # Errors
    ///
    /// Returns an error if the batch size is invalid (not 2-50 orders).
    pub fn build(self, token: &str) -> Result<BatchCancelRequest, BatchCancelError> {
        self.validate()?;

        let mut request = BatchCancelRequest::new(self.orders, token, self.req_id);

        if let Some(cl_ord_id) = self.cl_ord_id {
            request.params.cl_ord_id = Some(cl_ord_id);
        }

        Ok(request)
    }

    fn validate(&self) -> Result<(), BatchCancelError> {
        if self.orders.len() < MIN_BATCH_CANCEL_SIZE {
            return Err(BatchCancelError::TooFewOrders {
                count: self.orders.len(),
                min: MIN_BATCH_CANCEL_SIZE,
            });
        }
        if self.orders.len() > MAX_BATCH_CANCEL_SIZE {
            return Err(BatchCancelError::TooManyOrders {
                count: self.orders.len(),
                max: MAX_BATCH_CANCEL_SIZE,
            });
        }
        Ok(())
    }
}

impl Default for BatchCancelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when building a batch_cancel request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchCancelError {
    /// Too few orders in the batch.
    TooFewOrders { count: usize, min: usize },
    /// Too many orders in the batch.
    TooManyOrders { count: usize, max: usize },
}

impl std::fmt::Display for BatchCancelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewOrders { count, min } => {
                write!(f, "batch has {} orders, minimum is {}", count, min)
            }
            Self::TooManyOrders { count, max } => {
                write!(f, "batch has {} orders, maximum is {}", count, max)
            }
        }
    }
}

impl std::error::Error for BatchCancelError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_batch_cancel_request() {
        let request = BatchCancelRequest::new(
            vec![
                "1".to_string(),
                "2".to_string(),
                "ORDERX-IDXXX-XXXXX3".to_string(),
            ],
            "test_token",
            Some(1234567890),
        );

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "batch_cancel");
        assert_eq!(value["req_id"], 1234567890);
        assert_eq!(value["params"]["token"], "test_token");
        assert_eq!(value["params"]["orders"].as_array().unwrap().len(), 3);
        assert_eq!(value["params"]["orders"][0], "1");
        assert_eq!(value["params"]["orders"][1], "2");
        assert_eq!(value["params"]["orders"][2], "ORDERX-IDXXX-XXXXX3");
        assert!(value["params"].get("cl_ord_id").is_none());
    }

    #[test]
    fn serialize_batch_cancel_without_req_id() {
        let request =
            BatchCancelRequest::new(vec!["1".to_string(), "2".to_string()], "test_token", None);

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "batch_cancel");
        assert!(value.get("req_id").is_none());
    }

    #[test]
    fn serialize_batch_cancel_with_cl_ord_id() {
        let request = BatchCancelRequest::new(
            vec!["1".to_string(), "2".to_string()],
            "test_token",
            Some(42),
        )
        .with_cl_ord_id(vec!["client-1".to_string(), "client-2".to_string()]);

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["params"]["cl_ord_id"][0], "client-1");
        assert_eq!(value["params"]["cl_ord_id"][1], "client-2");
    }

    #[test]
    fn builder_validates_min_orders() {
        let result = BatchCancelBuilder::new().add_order("1").build("token");

        assert!(matches!(
            result,
            Err(BatchCancelError::TooFewOrders { count: 1, min: 2 })
        ));
    }

    #[test]
    fn builder_validates_max_orders() {
        let mut builder = BatchCancelBuilder::new();
        for i in 0..51 {
            builder = builder.add_order(&i.to_string());
        }
        let result = builder.build("token");

        assert!(matches!(
            result,
            Err(BatchCancelError::TooManyOrders { count: 51, max: 50 })
        ));
    }

    #[test]
    fn builder_accepts_valid_batch() {
        let result = BatchCancelBuilder::with_orders(vec![
            "1".to_string(),
            "2".to_string(),
            "ORDERX-IDXXX-XXXXX3".to_string(),
        ])
        .with_req_id(42)
        .build("token");

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.order_count(), 3);
        assert_eq!(request.req_id(), Some(42));
    }

    #[test]
    fn builder_with_cl_ord_id() {
        let result = BatchCancelBuilder::with_orders(vec!["1".to_string(), "2".to_string()])
            .with_cl_ord_id(vec!["client-1".to_string(), "client-2".to_string()])
            .build("token");

        assert!(result.is_ok());
        let json = serde_json::to_string(&result.unwrap()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["params"]["cl_ord_id"][0], "client-1");
    }

    #[test]
    fn builder_empty_orders_rejected() {
        let result = BatchCancelBuilder::new().build("token");

        assert!(matches!(
            result,
            Err(BatchCancelError::TooFewOrders { count: 0, min: 2 })
        ));
    }

    #[test]
    fn deserialize_success_response() {
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

        let response: BatchCancelResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.req_id, Some(1234567890));
        assert_eq!(response.result.as_ref().unwrap().count, 3);
    }

    #[test]
    fn deserialize_success_response_zero_cancelled() {
        let json = r#"{
            "method": "batch_cancel",
            "result": {
                "count": 0
            },
            "success": true,
            "time_in": "2022-06-13T08:09:10.123456Z",
            "time_out": "2022-06-13T08:09:10.789012Z"
        }"#;

        let response: BatchCancelResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.result.as_ref().unwrap().count, 0);
    }

    #[test]
    fn deserialize_error_response() {
        let json = r#"{
            "method": "batch_cancel",
            "success": false,
            "error": "EGeneral:Permission denied",
            "time_in": "2022-06-13T08:09:10.123456Z",
            "time_out": "2022-06-13T08:09:10.789012Z"
        }"#;

        let response: BatchCancelResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(
            response.error,
            Some("EGeneral:Permission denied".to_string())
        );
        assert!(response.result.is_none());
    }
}
