//! Amend order RPC models.
//!
//! Provides types for modifying existing orders via the Kraken WebSocket V2 API.
//! Unlike subscription channels, `amend_order` is an RPC-style one-shot
//! request/response command that preserves queue priority where applicable.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Price type for limit and trigger price fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(frozen, eq, eq_int, hash, from_py_object)
)]
pub enum PriceType {
    /// Absolute price value (default).
    Static,
    /// Percentage offset from reference.
    Pct,
    /// Price in quote currency terms.
    Quote,
}

/// Parameters for the amend_order request.
#[derive(Debug, Clone, Serialize)]
pub struct AmendOrderParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cl_ord_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_qty: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_price_type: Option<PriceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_qty: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_price: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_price_type: Option<PriceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    pub token: super::RedactedToken,
}

/// The amend_order request message.
#[derive(Debug, Clone, Serialize)]
pub struct AmendOrderRequest {
    method: String,
    params: AmendOrderParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    req_id: Option<u64>,
}

impl AmendOrderRequest {
    /// Creates a new amend_order request from params and optional request ID.
    #[must_use]
    pub fn new(params: AmendOrderParams, req_id: Option<u64>) -> Self {
        Self {
            method: "amend_order".to_string(),
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

/// Successful order amendment result.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct AmendOrderResult {
    pub amend_id: String,
    #[serde(default)]
    pub order_id: Option<String>,
    #[serde(default)]
    pub cl_ord_id: Option<String>,
}

/// Response to an amend_order request.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct AmendOrderResponse {
    pub method: String,
    pub success: bool,
    #[serde(default)]
    pub result: Option<AmendOrderResult>,
    #[serde(default)]
    pub error: Option<String>,
    pub time_in: String,
    pub time_out: String,
    #[serde(default)]
    pub req_id: Option<u64>,
}

/// Builder for constructing amend_order requests with validation.
#[derive(Debug, Clone)]
pub struct AmendOrderBuilder {
    order_id: Option<String>,
    cl_ord_id: Option<String>,
    order_qty: Option<Decimal>,
    limit_price: Option<Decimal>,
    limit_price_type: Option<PriceType>,
    display_qty: Option<Decimal>,
    post_only: Option<bool>,
    trigger_price: Option<Decimal>,
    trigger_price_type: Option<PriceType>,
    symbol: Option<String>,
    deadline: Option<String>,
    req_id: Option<u64>,
}

impl AmendOrderBuilder {
    /// Creates a new builder targeting an order by its Kraken order ID.
    #[must_use]
    pub fn by_order_id(order_id: &str) -> Self {
        Self {
            order_id: Some(order_id.to_string()),
            cl_ord_id: None,
            order_qty: None,
            limit_price: None,
            limit_price_type: None,
            display_qty: None,
            post_only: None,
            trigger_price: None,
            trigger_price_type: None,
            symbol: None,
            deadline: None,
            req_id: None,
        }
    }

    /// Creates a new builder targeting an order by its client order ID.
    #[must_use]
    pub fn by_cl_ord_id(cl_ord_id: &str) -> Self {
        Self {
            order_id: None,
            cl_ord_id: Some(cl_ord_id.to_string()),
            order_qty: None,
            limit_price: None,
            limit_price_type: None,
            display_qty: None,
            post_only: None,
            trigger_price: None,
            trigger_price_type: None,
            symbol: None,
            deadline: None,
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

    /// Sets the limit price type.
    #[must_use]
    pub fn with_limit_price_type(mut self, price_type: PriceType) -> Self {
        self.limit_price_type = Some(price_type);
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

    /// Sets the new trigger price.
    #[must_use]
    pub fn with_trigger_price(mut self, price: Decimal) -> Self {
        self.trigger_price = Some(price);
        self
    }

    /// Sets the trigger price type.
    #[must_use]
    pub fn with_trigger_price_type(mut self, price_type: PriceType) -> Self {
        self.trigger_price_type = Some(price_type);
        self
    }

    /// Sets the symbol (required for non-crypto pairs).
    #[must_use]
    pub fn with_symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    /// Sets the deadline (RFC3339 timestamp).
    #[must_use]
    pub fn with_deadline(mut self, deadline: &str) -> Self {
        self.deadline = Some(deadline.to_string());
        self
    }

    /// Sets the request ID for correlation.
    #[must_use]
    pub fn with_req_id(mut self, req_id: u64) -> Self {
        self.req_id = Some(req_id);
        self
    }

    /// Validates and builds the amend order parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if no order identifier is set or no amendment fields are provided.
    pub fn build(self, token: &str) -> Result<AmendOrderParams, AmendOrderError> {
        self.validate()?;

        Ok(AmendOrderParams {
            order_id: self.order_id,
            cl_ord_id: self.cl_ord_id,
            order_qty: self.order_qty,
            limit_price: self.limit_price,
            limit_price_type: self.limit_price_type,
            display_qty: self.display_qty,
            post_only: self.post_only,
            trigger_price: self.trigger_price,
            trigger_price_type: self.trigger_price_type,
            symbol: self.symbol,
            deadline: self.deadline,
            token: super::RedactedToken::new(token),
        })
    }

    /// Validates and builds the full request.
    ///
    /// # Errors
    ///
    /// Returns an error if no order identifier is set or no amendment fields are provided.
    pub fn build_request(self, token: &str) -> Result<AmendOrderRequest, AmendOrderError> {
        let req_id = self.req_id;
        let params = self.build(token)?;
        Ok(AmendOrderRequest::new(params, req_id))
    }

    fn validate(&self) -> Result<(), AmendOrderError> {
        if self.order_id.is_none() && self.cl_ord_id.is_none() {
            return Err(AmendOrderError::MissingOrderIdentifier);
        }

        if self.order_qty.is_none()
            && self.limit_price.is_none()
            && self.display_qty.is_none()
            && self.post_only.is_none()
            && self.trigger_price.is_none()
        {
            return Err(AmendOrderError::NoAmendmentFields);
        }

        Ok(())
    }
}

/// Errors that can occur when building an amend_order request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmendOrderError {
    /// At least one of order_id or cl_ord_id must be provided.
    MissingOrderIdentifier,
    /// At least one amendment field must be set.
    NoAmendmentFields,
}

impl std::fmt::Display for AmendOrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingOrderIdentifier => {
                write!(f, "at least one of order_id or cl_ord_id is required")
            }
            Self::NoAmendmentFields => {
                write!(f, "at least one amendment field must be set")
            }
        }
    }
}

impl std::error::Error for AmendOrderError {}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn serialize_amend_by_order_id() {
        let params = AmendOrderBuilder::by_order_id("OAIYAU-LGI3M-PFM5VW")
            .with_limit_price(dec!(61031.3))
            .build("test_token")
            .unwrap();

        let request = AmendOrderRequest::new(params, Some(42));
        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "amend_order");
        assert_eq!(value["req_id"], 42);
        assert_eq!(value["params"]["order_id"], "OAIYAU-LGI3M-PFM5VW");
        assert_eq!(value["params"]["limit_price"], "61031.3");
        assert_eq!(value["params"]["token"], "test_token");
        assert!(value["params"].get("cl_ord_id").is_none());
    }

    #[test]
    fn serialize_amend_by_cl_ord_id() {
        let params = AmendOrderBuilder::by_cl_ord_id("2c6be801-1f53-4f79-a0bb-4ea1c95dfae9")
            .with_limit_price(dec!(490795))
            .with_order_qty(dec!(1.2))
            .build("test_token")
            .unwrap();

        let request = AmendOrderRequest::new(params, None);
        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "amend_order");
        assert_eq!(
            value["params"]["cl_ord_id"],
            "2c6be801-1f53-4f79-a0bb-4ea1c95dfae9"
        );
        assert_eq!(value["params"]["limit_price"], "490795");
        assert_eq!(value["params"]["order_qty"], "1.2");
        assert!(value.get("req_id").is_none());
        assert!(value["params"].get("order_id").is_none());
    }

    #[test]
    fn serialize_amend_with_all_fields() {
        let params = AmendOrderBuilder::by_order_id("OAIYAU-LGI3M-PFM5VW")
            .with_order_qty(dec!(1.5))
            .with_limit_price(dec!(61000))
            .with_limit_price_type(PriceType::Static)
            .with_display_qty(dec!(0.5))
            .with_post_only(true)
            .with_trigger_price(dec!(60000))
            .with_trigger_price_type(PriceType::Pct)
            .with_symbol("BTC/USD")
            .with_deadline("2024-07-21T09:53:59.050Z")
            .build("test_token")
            .unwrap();

        let request = AmendOrderRequest::new(params, Some(99));
        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["params"]["order_qty"], "1.5");
        assert_eq!(value["params"]["limit_price"], "61000");
        assert_eq!(value["params"]["limit_price_type"], "static");
        assert_eq!(value["params"]["display_qty"], "0.5");
        assert_eq!(value["params"]["post_only"], true);
        assert_eq!(value["params"]["trigger_price"], "60000");
        assert_eq!(value["params"]["trigger_price_type"], "pct");
        assert_eq!(value["params"]["symbol"], "BTC/USD");
        assert_eq!(value["params"]["deadline"], "2024-07-21T09:53:59.050Z");
        assert_eq!(value["req_id"], 99);
    }

    #[test]
    fn serialize_price_type_quote() {
        let params = AmendOrderBuilder::by_order_id("OTEST")
            .with_limit_price(dec!(100))
            .with_limit_price_type(PriceType::Quote)
            .build("token")
            .unwrap();

        let json = serde_json::to_string(&AmendOrderRequest::new(params, None)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["params"]["limit_price_type"], "quote");
    }

    #[test]
    fn validate_missing_identifier() {
        let builder = AmendOrderBuilder {
            order_id: None,
            cl_ord_id: None,
            order_qty: Some(dec!(1.0)),
            limit_price: None,
            limit_price_type: None,
            display_qty: None,
            post_only: None,
            trigger_price: None,
            trigger_price_type: None,
            symbol: None,
            deadline: None,
            req_id: None,
        };

        let result = builder.build("token");
        assert!(matches!(
            result,
            Err(AmendOrderError::MissingOrderIdentifier)
        ));
    }

    #[test]
    fn validate_no_amendment_fields() {
        let result = AmendOrderBuilder::by_order_id("OTEST").build("token");

        assert!(matches!(result, Err(AmendOrderError::NoAmendmentFields)));
    }

    #[test]
    fn deserialize_success_response() {
        let json = r#"{
            "method": "amend_order",
            "result": {
                "amend_id": "TTW6PD-RC36L-ZZSWNU",
                "cl_ord_id": "2c6be801-1f53-4f79-a0bb-4ea1c95dfae9"
            },
            "success": true,
            "time_in": "2024-07-26T13:39:04.922699Z",
            "time_out": "2024-07-26T13:39:04.924912Z"
        }"#;

        let response: AmendOrderResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(
            response.result.as_ref().unwrap().amend_id,
            "TTW6PD-RC36L-ZZSWNU"
        );
        assert_eq!(
            response.result.as_ref().unwrap().cl_ord_id,
            Some("2c6be801-1f53-4f79-a0bb-4ea1c95dfae9".to_string())
        );
        assert!(response.result.as_ref().unwrap().order_id.is_none());
        assert!(response.req_id.is_none());
    }

    #[test]
    fn deserialize_error_response() {
        let json = r#"{
            "method": "amend_order",
            "error": "EOrder:Unknown order",
            "success": false,
            "time_in": "2024-07-26T13:39:04.922699Z",
            "time_out": "2024-07-26T13:39:04.924912Z"
        }"#;

        let response: AmendOrderResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(response.error, Some("EOrder:Unknown order".to_string()));
        assert!(response.result.is_none());
    }
}
