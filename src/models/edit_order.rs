//! Edit order RPC models.
//!
//! Provides types for modifying live orders via the Kraken WebSocket V2 API.
//! When successful, the original order is cancelled and a new order is created
//! with the adjusted parameters and a new `order_id`.
//!
//! # Limitations
//!
//! - Triggered stop-loss or take-profit orders are not supported
//! - Orders with conditional close terms attached are not supported
//! - Rejected if executed volume exceeds new volume
//! - No `cl_ord_id` support
//! - Queue position is lost
//!
//! # Note
//!
//! The `amend_order` endpoint is the preferred alternative with fewer
//! restrictions and better performance.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::{FeeCurrencyPreference, TriggerParams};

/// Parameters for the edit_order request.
#[derive(Debug, Clone, Serialize)]
pub struct EditOrderParams {
    /// Original order identifier to edit.
    pub order_id: String,
    /// Trading pair (cannot be changed, but must be specified).
    pub symbol: String,
    pub token: super::RedactedToken,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_qty: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_qty: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_preference: Option<FeeCurrencyPreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_userref: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggers: Option<TriggerParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<bool>,
}

/// The edit_order request message.
#[derive(Debug, Clone, Serialize)]
pub struct EditOrderRequest {
    method: String,
    params: EditOrderParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    req_id: Option<u64>,
}

impl EditOrderRequest {
    /// Creates a new edit_order request.
    #[must_use]
    pub fn new(params: EditOrderParams, req_id: Option<u64>) -> Self {
        Self {
            method: "edit_order".to_string(),
            params,
            req_id,
        }
    }

    /// Returns the request ID if set.
    #[must_use]
    pub fn req_id(&self) -> Option<u64> {
        self.req_id
    }

    /// Returns the order ID being edited.
    #[must_use]
    pub fn order_id(&self) -> &str {
        &self.params.order_id
    }
}

/// Successful edit_order result.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct EditOrderResult {
    /// New order ID (the edited order).
    pub order_id: String,
    /// Original order ID that was cancelled.
    pub original_order_id: String,
}

/// Response to an edit_order request.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct EditOrderResponse {
    pub method: String,
    pub success: bool,
    #[serde(default)]
    pub result: Option<EditOrderResult>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub time_in: Option<String>,
    #[serde(default)]
    pub time_out: Option<String>,
    #[serde(default)]
    pub req_id: Option<u64>,
}

/// Builder for constructing edit_order requests with validation.
#[derive(Debug, Clone)]
pub struct EditOrderBuilder {
    order_id: String,
    symbol: String,
    order_qty: Option<Decimal>,
    limit_price: Option<Decimal>,
    display_qty: Option<Decimal>,
    post_only: Option<bool>,
    reduce_only: Option<bool>,
    fee_preference: Option<FeeCurrencyPreference>,
    order_userref: Option<i64>,
    deadline: Option<String>,
    triggers: Option<TriggerParams>,
    validate: Option<bool>,
    req_id: Option<u64>,
}

impl EditOrderBuilder {
    /// Creates a new builder for editing an order.
    ///
    /// # Arguments
    ///
    /// * `order_id` - The original order ID to edit
    /// * `symbol` - The trading pair (must match the original order)
    #[must_use]
    pub fn new(order_id: &str, symbol: &str) -> Self {
        Self {
            order_id: order_id.to_string(),
            symbol: symbol.to_string(),
            order_qty: None,
            limit_price: None,
            display_qty: None,
            post_only: None,
            reduce_only: None,
            fee_preference: None,
            order_userref: None,
            deadline: None,
            triggers: None,
            validate: None,
            req_id: None,
        }
    }

    /// Sets the new order quantity.
    #[must_use]
    pub fn with_order_qty(mut self, qty: Decimal) -> Self {
        self.order_qty = Some(qty);
        self
    }

    /// Sets the new limit price.
    #[must_use]
    pub fn with_limit_price(mut self, price: Decimal) -> Self {
        self.limit_price = Some(price);
        self
    }

    /// Sets the display quantity for iceberg orders.
    #[must_use]
    pub fn with_display_qty(mut self, qty: Decimal) -> Self {
        self.display_qty = Some(qty);
        self
    }

    /// Sets the post-only flag.
    #[must_use]
    pub fn with_post_only(mut self, post_only: bool) -> Self {
        self.post_only = Some(post_only);
        self
    }

    /// Sets the reduce-only flag.
    #[must_use]
    pub fn with_reduce_only(mut self, reduce_only: bool) -> Self {
        self.reduce_only = Some(reduce_only);
        self
    }

    /// Sets the fee currency preference.
    #[must_use]
    pub fn with_fee_preference(mut self, pref: FeeCurrencyPreference) -> Self {
        self.fee_preference = Some(pref);
        self
    }

    /// Sets the user reference.
    #[must_use]
    pub fn with_order_userref(mut self, userref: i64) -> Self {
        self.order_userref = Some(userref);
        self
    }

    /// Sets the deadline (RFC3339 timestamp, 500ms-60s from now).
    #[must_use]
    pub fn with_deadline(mut self, deadline: &str) -> Self {
        self.deadline = Some(deadline.to_string());
        self
    }

    /// Sets trigger parameters.
    #[must_use]
    pub fn with_triggers(mut self, triggers: TriggerParams) -> Self {
        self.triggers = Some(triggers);
        self
    }

    /// Sets validate mode (dry-run without actually editing).
    #[must_use]
    pub fn with_validate(mut self, validate: bool) -> Self {
        self.validate = Some(validate);
        self
    }

    /// Sets the request ID for correlation.
    #[must_use]
    pub fn with_req_id(mut self, req_id: u64) -> Self {
        self.req_id = Some(req_id);
        self
    }

    /// Validates and builds the edit order request.
    ///
    /// # Errors
    ///
    /// Returns an error if no editable fields are provided.
    pub fn build(self, token: &str) -> Result<EditOrderRequest, EditOrderError> {
        self.validate()?;

        let params = EditOrderParams {
            order_id: self.order_id,
            symbol: self.symbol,
            token: super::RedactedToken::new(token),
            order_qty: self.order_qty,
            limit_price: self.limit_price,
            display_qty: self.display_qty,
            post_only: self.post_only,
            reduce_only: self.reduce_only,
            fee_preference: self.fee_preference,
            order_userref: self.order_userref,
            deadline: self.deadline,
            triggers: self.triggers,
            validate: self.validate,
        };

        Ok(EditOrderRequest::new(params, self.req_id))
    }

    fn validate(&self) -> Result<(), EditOrderError> {
        // At least one editable field must be provided
        let has_edit = self.order_qty.is_some()
            || self.limit_price.is_some()
            || self.display_qty.is_some()
            || self.post_only.is_some()
            || self.reduce_only.is_some()
            || self.fee_preference.is_some()
            || self.order_userref.is_some()
            || self.triggers.is_some();

        if !has_edit {
            return Err(EditOrderError::NoEditFields);
        }

        Ok(())
    }
}

/// Errors that can occur when building an edit_order request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOrderError {
    /// At least one editable field must be provided.
    NoEditFields,
}

impl std::fmt::Display for EditOrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEditFields => write!(
                f,
                "at least one editable field (order_qty, limit_price, etc.) must be provided"
            ),
        }
    }
}

impl std::error::Error for EditOrderError {}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn serialize_edit_order_request() {
        let request = EditOrderBuilder::new("OFGKYQ-FHPCQ-HUQFEK", "BTC/USD")
            .with_order_qty(dec!(0.5))
            .with_limit_price(dec!(52000))
            .with_req_id(123456789)
            .build("test_token")
            .unwrap();

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "edit_order");
        assert_eq!(value["req_id"], 123456789);
        assert_eq!(value["params"]["order_id"], "OFGKYQ-FHPCQ-HUQFEK");
        assert_eq!(value["params"]["symbol"], "BTC/USD");
        assert_eq!(value["params"]["token"], "test_token");
        assert_eq!(value["params"]["order_qty"], "0.5");
        assert_eq!(value["params"]["limit_price"], "52000");
    }

    #[test]
    fn serialize_edit_order_with_all_fields() {
        let request = EditOrderBuilder::new("ORDER123", "ETH/USD")
            .with_order_qty(dec!(1.5))
            .with_limit_price(dec!(2500))
            .with_display_qty(dec!(0.5))
            .with_post_only(true)
            .with_reduce_only(false)
            .with_fee_preference(FeeCurrencyPreference::Quote)
            .with_order_userref(42)
            .with_deadline("2024-01-15T12:00:00Z")
            .with_validate(true)
            .build("token")
            .unwrap();

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["params"]["display_qty"], "0.5");
        assert_eq!(value["params"]["post_only"], true);
        assert_eq!(value["params"]["reduce_only"], false);
        assert_eq!(value["params"]["fee_preference"], "quote");
        assert_eq!(value["params"]["order_userref"], 42);
        assert_eq!(value["params"]["deadline"], "2024-01-15T12:00:00Z");
        assert_eq!(value["params"]["validate"], true);
    }

    #[test]
    fn validate_requires_edit_field() {
        let result = EditOrderBuilder::new("ORDER123", "BTC/USD").build("token");

        assert!(matches!(result, Err(EditOrderError::NoEditFields)));
    }

    #[test]
    fn deserialize_success_response() {
        let json = r#"{
            "method": "edit_order",
            "success": true,
            "result": {
                "order_id": "ORDERX-IDXXX-XXXXX2",
                "original_order_id": "ORDERX-IDXXX-XXXXX1"
            },
            "req_id": 1234567890,
            "time_in": "2022-07-15T12:56:09.876488Z",
            "time_out": "2022-07-15T12:56:09.923422Z"
        }"#;

        let response: EditOrderResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.req_id, Some(1234567890));
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert_eq!(result.order_id, "ORDERX-IDXXX-XXXXX2");
        assert_eq!(result.original_order_id, "ORDERX-IDXXX-XXXXX1");
    }

    #[test]
    fn deserialize_error_response() {
        let json = r#"{
            "method": "edit_order",
            "success": false,
            "error": "EOrder:Unknown order",
            "req_id": 123
        }"#;

        let response: EditOrderResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(response.error, Some("EOrder:Unknown order".to_string()));
        assert!(response.result.is_none());
    }

    #[test]
    fn request_accessors() {
        let request = EditOrderBuilder::new("ORDER123", "BTC/USD")
            .with_limit_price(dec!(50000))
            .with_req_id(42)
            .build("token")
            .unwrap();

        assert_eq!(request.order_id(), "ORDER123");
        assert_eq!(request.req_id(), Some(42));
    }
}
