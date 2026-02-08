//! Batch add orders RPC models.
//!
//! Provides types for placing multiple orders at once via the Kraken WebSocket V2 API.
//! All orders in a batch must target the same currency pair.
//!
//! # Batch Rules
//!
//! - Minimum 2 orders, maximum 15 orders per batch
//! - All orders must be for the same symbol
//! - If validation fails for any order, the entire batch is rejected
//! - If an order fails engine pre-match checks (e.g., funding), that order is
//!   rejected but others may still process

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::{
    ConditionalOrder, FeeCurrencyPreference, OrderSide, OrderType, StpType, TimeInForce,
    TriggerParams,
};

/// Minimum number of orders in a batch.
pub const MIN_BATCH_SIZE: usize = 2;

/// Maximum number of orders in a batch.
pub const MAX_BATCH_SIZE: usize = 15;

/// A single order entry within a batch.
///
/// Similar to `AddOrderParams` but without `symbol` and `token` which are
/// specified at the batch level.
#[derive(Debug, Clone, Serialize)]
pub struct BatchOrderEntry {
    pub order_type: OrderType,
    pub side: OrderSide,
    pub order_qty: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cl_ord_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_userref: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggers: Option<TriggerParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional: Option<ConditionalOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_qty: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stp_type: Option<StpType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_preference: Option<FeeCurrencyPreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cash_order_qty: Option<Decimal>,
}

impl BatchOrderEntry {
    /// Creates a new market order entry.
    #[must_use]
    pub fn market(side: OrderSide, qty: Decimal) -> Self {
        Self::new(OrderType::Market, side, qty)
    }

    /// Creates a new limit order entry.
    #[must_use]
    pub fn limit(side: OrderSide, qty: Decimal, price: Decimal) -> Self {
        let mut entry = Self::new(OrderType::Limit, side, qty);
        entry.limit_price = Some(price);
        entry
    }

    /// Creates a new order entry with the specified type.
    #[must_use]
    pub fn new(order_type: OrderType, side: OrderSide, qty: Decimal) -> Self {
        Self {
            order_type,
            side,
            order_qty: qty,
            limit_price: None,
            time_in_force: None,
            expire_time: None,
            post_only: None,
            reduce_only: None,
            margin: None,
            cl_ord_id: None,
            order_userref: None,
            triggers: None,
            conditional: None,
            display_qty: None,
            stp_type: None,
            fee_preference: None,
            cash_order_qty: None,
        }
    }

    /// Sets the limit price.
    #[must_use]
    pub fn with_limit_price(mut self, price: Decimal) -> Self {
        self.limit_price = Some(price);
        self
    }

    /// Sets the time in force.
    #[must_use]
    pub fn with_time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = Some(tif);
        self
    }

    /// Sets the expiration time (required for GTD orders).
    #[must_use]
    pub fn with_expire_time(mut self, expire: &str) -> Self {
        self.expire_time = Some(expire.to_string());
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

    /// Sets the margin flag.
    #[must_use]
    pub fn with_margin(mut self, margin: bool) -> Self {
        self.margin = Some(margin);
        self
    }

    /// Sets the client order ID.
    #[must_use]
    pub fn with_cl_ord_id(mut self, cl_ord_id: &str) -> Self {
        self.cl_ord_id = Some(cl_ord_id.to_string());
        self
    }

    /// Sets the user reference.
    #[must_use]
    pub fn with_order_userref(mut self, userref: i64) -> Self {
        self.order_userref = Some(userref);
        self
    }

    /// Sets trigger parameters for stop/take-profit orders.
    #[must_use]
    pub fn with_triggers(mut self, triggers: TriggerParams) -> Self {
        self.triggers = Some(triggers);
        self
    }

    /// Sets a conditional (secondary) order.
    #[must_use]
    pub fn with_conditional(mut self, conditional: ConditionalOrder) -> Self {
        self.conditional = Some(conditional);
        self
    }

    /// Sets the display quantity for iceberg orders.
    #[must_use]
    pub fn with_display_qty(mut self, qty: Decimal) -> Self {
        self.display_qty = Some(qty);
        self
    }

    /// Sets the self-trade prevention type.
    #[must_use]
    pub fn with_stp_type(mut self, stp: StpType) -> Self {
        self.stp_type = Some(stp);
        self
    }

    /// Sets the fee currency preference.
    #[must_use]
    pub fn with_fee_preference(mut self, pref: FeeCurrencyPreference) -> Self {
        self.fee_preference = Some(pref);
        self
    }

    /// Sets the cash order quantity (for market orders in quote currency).
    #[must_use]
    pub fn with_cash_order_qty(mut self, qty: Decimal) -> Self {
        self.cash_order_qty = Some(qty);
        self
    }
}

/// Parameters for the batch_add request.
#[derive(Debug, Clone, Serialize)]
pub struct BatchAddParams {
    pub symbol: String,
    pub orders: Vec<BatchOrderEntry>,
    pub token: super::RedactedToken,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<bool>,
}

/// The batch_add request message.
#[derive(Debug, Clone, Serialize)]
pub struct BatchAddRequest {
    method: String,
    params: BatchAddParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    req_id: Option<u64>,
}

impl BatchAddRequest {
    /// Creates a new batch_add request.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Currency pair for all orders (e.g., "BTC/USD")
    /// * `orders` - Vector of 2-15 order entries
    /// * `token` - Authentication token
    /// * `req_id` - Optional request ID for correlation
    #[must_use]
    pub fn new(
        symbol: &str,
        orders: Vec<BatchOrderEntry>,
        token: &str,
        req_id: Option<u64>,
    ) -> Self {
        Self {
            method: "batch_add".to_string(),
            params: BatchAddParams {
                symbol: symbol.to_string(),
                orders,
                token: super::RedactedToken::new(token),
                deadline: None,
                validate: None,
            },
            req_id,
        }
    }

    /// Sets the deadline for the batch (RFC3339 timestamp).
    #[must_use]
    pub fn with_deadline(mut self, deadline: &str) -> Self {
        self.params.deadline = Some(deadline.to_string());
        self
    }

    /// Sets validate mode (dry-run without placing orders).
    #[must_use]
    pub fn with_validate(mut self, validate: bool) -> Self {
        self.params.validate = Some(validate);
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

/// Result for a single order in the batch response.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchAddOrderResult {
    pub order_id: String,
    #[serde(default)]
    pub cl_ord_id: Option<String>,
    #[serde(default)]
    pub order_userref: Option<i64>,
}

/// Response to a batch_add request.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchAddResponse {
    pub method: String,
    pub success: bool,
    #[serde(default)]
    pub result: Option<Vec<BatchAddOrderResult>>,
    #[serde(default)]
    pub error: Option<String>,
    pub time_in: String,
    pub time_out: String,
    #[serde(default)]
    pub req_id: Option<u64>,
}

/// Builder for constructing batch_add requests with validation.
#[derive(Debug, Clone)]
pub struct BatchAddBuilder {
    symbol: String,
    orders: Vec<BatchOrderEntry>,
    deadline: Option<String>,
    validate: Option<bool>,
    req_id: Option<u64>,
}

impl BatchAddBuilder {
    /// Creates a new builder for a batch of orders.
    #[must_use]
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            orders: Vec::new(),
            deadline: None,
            validate: None,
            req_id: None,
        }
    }

    /// Adds an order to the batch.
    #[must_use]
    pub fn add_order(mut self, order: BatchOrderEntry) -> Self {
        self.orders.push(order);
        self
    }

    /// Adds multiple orders to the batch.
    #[must_use]
    pub fn add_orders(mut self, orders: Vec<BatchOrderEntry>) -> Self {
        self.orders.extend(orders);
        self
    }

    /// Sets the deadline for the batch (RFC3339 timestamp).
    #[must_use]
    pub fn with_deadline(mut self, deadline: &str) -> Self {
        self.deadline = Some(deadline.to_string());
        self
    }

    /// Sets validate mode (dry-run without placing orders).
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

    /// Validates and builds the batch request.
    ///
    /// # Errors
    ///
    /// Returns an error if the batch size is invalid (not 2-15 orders).
    pub fn build(self, token: &str) -> Result<BatchAddRequest, BatchAddError> {
        self.validate()?;

        let mut request = BatchAddRequest::new(&self.symbol, self.orders, token, self.req_id);

        if let Some(deadline) = self.deadline {
            request.params.deadline = Some(deadline);
        }
        if let Some(validate) = self.validate {
            request.params.validate = Some(validate);
        }

        Ok(request)
    }

    fn validate(&self) -> Result<(), BatchAddError> {
        if self.orders.len() < MIN_BATCH_SIZE {
            return Err(BatchAddError::TooFewOrders {
                count: self.orders.len(),
                min: MIN_BATCH_SIZE,
            });
        }
        if self.orders.len() > MAX_BATCH_SIZE {
            return Err(BatchAddError::TooManyOrders {
                count: self.orders.len(),
                max: MAX_BATCH_SIZE,
            });
        }
        Ok(())
    }
}

/// Errors that can occur when building a batch_add request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchAddError {
    /// Too few orders in the batch.
    TooFewOrders { count: usize, min: usize },
    /// Too many orders in the batch.
    TooManyOrders { count: usize, max: usize },
}

impl std::fmt::Display for BatchAddError {
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

impl std::error::Error for BatchAddError {}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn serialize_batch_add_request() {
        let orders = vec![
            BatchOrderEntry::limit(OrderSide::Buy, dec!(0.1), dec!(50000)).with_order_userref(1),
            BatchOrderEntry::limit(OrderSide::Sell, dec!(0.2), dec!(55000))
                .with_order_userref(2)
                .with_stp_type(StpType::CancelBoth),
        ];

        let request = BatchAddRequest::new("BTC/USD", orders, "test_token", Some(123));

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "batch_add");
        assert_eq!(value["req_id"], 123);
        assert_eq!(value["params"]["symbol"], "BTC/USD");
        assert_eq!(value["params"]["token"], "test_token");
        assert_eq!(value["params"]["orders"].as_array().unwrap().len(), 2);
        assert_eq!(value["params"]["orders"][0]["side"], "buy");
        assert_eq!(value["params"]["orders"][0]["order_type"], "limit");
        assert_eq!(value["params"]["orders"][0]["limit_price"], "50000");
        assert_eq!(value["params"]["orders"][1]["stp_type"], "cancel_both");
    }

    #[test]
    fn serialize_batch_with_deadline_and_validate() {
        let orders = vec![
            BatchOrderEntry::market(OrderSide::Buy, dec!(1)),
            BatchOrderEntry::market(OrderSide::Sell, dec!(1)),
        ];

        let request = BatchAddRequest::new("ETH/USD", orders, "token", None)
            .with_deadline("2024-01-15T12:00:00Z")
            .with_validate(true);

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["params"]["deadline"], "2024-01-15T12:00:00Z");
        assert_eq!(value["params"]["validate"], true);
    }

    #[test]
    fn builder_validates_min_orders() {
        let result = BatchAddBuilder::new("BTC/USD")
            .add_order(BatchOrderEntry::market(OrderSide::Buy, dec!(1)))
            .build("token");

        assert!(matches!(
            result,
            Err(BatchAddError::TooFewOrders { count: 1, min: 2 })
        ));
    }

    #[test]
    fn builder_validates_max_orders() {
        let mut builder = BatchAddBuilder::new("BTC/USD");
        for _ in 0..16 {
            builder = builder.add_order(BatchOrderEntry::market(OrderSide::Buy, dec!(1)));
        }
        let result = builder.build("token");

        assert!(matches!(
            result,
            Err(BatchAddError::TooManyOrders { count: 16, max: 15 })
        ));
    }

    #[test]
    fn builder_accepts_valid_batch() {
        let result = BatchAddBuilder::new("BTC/USD")
            .add_order(BatchOrderEntry::limit(OrderSide::Buy, dec!(1), dec!(50000)))
            .add_order(BatchOrderEntry::limit(
                OrderSide::Sell,
                dec!(1),
                dec!(55000),
            ))
            .with_req_id(42)
            .build("token");

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.order_count(), 2);
        assert_eq!(request.req_id(), Some(42));
    }

    #[test]
    fn deserialize_success_response() {
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

        let response: BatchAddResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(response.req_id, Some(1234567890));
        assert!(response.result.is_some());

        let results = response.result.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].order_id, "ORDERX-IDXXX-XXXXX1");
        assert_eq!(results[0].cl_ord_id, Some("my-order-1".to_string()));
        assert_eq!(results[0].order_userref, Some(1));
        assert_eq!(results[1].order_id, "ORDERX-IDXXX-XXXXX2");
        assert!(results[1].cl_ord_id.is_none());
    }

    #[test]
    fn deserialize_error_response() {
        let json = r#"{
            "method": "batch_add",
            "success": false,
            "error": "EOrder:Invalid order",
            "time_in": "2022-06-13T08:09:10.123456Z",
            "time_out": "2022-06-13T08:09:10.789012Z"
        }"#;

        let response: BatchAddResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(response.error, Some("EOrder:Invalid order".to_string()));
        assert!(response.result.is_none());
    }

    #[test]
    fn order_entry_builder_methods() {
        let order = BatchOrderEntry::new(OrderType::Limit, OrderSide::Buy, dec!(1))
            .with_limit_price(dec!(50000))
            .with_time_in_force(TimeInForce::Gtc)
            .with_post_only(true)
            .with_cl_ord_id("my-order")
            .with_order_userref(42);

        let json = serde_json::to_string(&order).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["order_type"], "limit");
        assert_eq!(value["side"], "buy");
        assert_eq!(value["limit_price"], "50000");
        assert_eq!(value["time_in_force"], "gtc");
        assert_eq!(value["post_only"], true);
        assert_eq!(value["cl_ord_id"], "my-order");
        assert_eq!(value["order_userref"], 42);
    }
}
