//! Add order RPC models.
//!
//! Provides types for placing orders via the Kraken WebSocket V2 API.
//! Unlike subscription channels, `add_order` is an RPC-style one-shot
//! request/response command.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Order type specifying how the order should be executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderType {
    Limit,
    Market,
    Iceberg,
    StopLoss,
    StopLossLimit,
    TakeProfit,
    TakeProfitLimit,
    TrailingStop,
    TrailingStopLimit,
    SettlePosition,
}

/// Order side (buy or sell).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Time in force specifying how long the order remains active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeInForce {
    /// Good 'til cancelled (default).
    Gtc,
    /// Good 'til date (requires `expire_time`).
    Gtd,
    /// Immediate or cancel.
    Ioc,
}

/// Price reference for trigger orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerReference {
    /// Last traded price.
    Last,
    /// Index price.
    Index,
}

/// Trigger price type (percentage or static).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerPriceType {
    /// Percentage offset from reference.
    Pct,
    /// Static price value.
    Static,
}

/// Self-trade prevention type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StpType {
    CancelNewest,
    CancelOldest,
    CancelBoth,
}

/// Trigger parameters for stop-loss, take-profit, and trailing orders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerParams {
    pub reference: TriggerReference,
    pub price: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_type: Option<TriggerPriceType>,
}

impl TriggerParams {
    /// Creates new trigger parameters with default static price type.
    #[must_use]
    pub fn new(reference: TriggerReference, price: Decimal) -> Self {
        Self {
            reference,
            price,
            price_type: None,
        }
    }

    /// Creates trigger parameters with percentage offset.
    #[must_use]
    pub fn percentage(reference: TriggerReference, price: Decimal) -> Self {
        Self {
            reference,
            price,
            price_type: Some(TriggerPriceType::Pct),
        }
    }
}

/// Conditional (secondary) order attached to the primary order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalOrder {
    pub order_type: OrderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_price: Option<Decimal>,
}

/// Fee preference for order execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeeCurrencyPreference {
    Base,
    Quote,
}

/// Parameters for the add_order request.
#[derive(Debug, Clone, Serialize)]
pub struct AddOrderParams {
    pub order_type: OrderType,
    pub side: OrderSide,
    pub symbol: String,
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
    pub validate: Option<bool>,
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
    pub no_mpp: Option<bool>,
    pub token: super::RedactedToken,
}

/// The add_order request message.
#[derive(Debug, Clone, Serialize)]
pub struct AddOrderRequest {
    method: String,
    params: AddOrderParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    req_id: Option<u64>,
}

impl AddOrderRequest {
    /// Creates a new add_order request from params and optional request ID.
    #[must_use]
    pub fn new(params: AddOrderParams, req_id: Option<u64>) -> Self {
        Self {
            method: "add_order".to_string(),
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

/// Successful order placement result.
#[derive(Debug, Clone, Deserialize)]
pub struct AddOrderResult {
    pub order_id: String,
    #[serde(default)]
    pub cl_ord_id: Option<String>,
    #[serde(default)]
    pub order_userref: Option<i64>,
}

/// Response to an add_order request.
#[derive(Debug, Clone, Deserialize)]
pub struct AddOrderResponse {
    pub method: String,
    pub success: bool,
    #[serde(default)]
    pub result: Option<AddOrderResult>,
    #[serde(default)]
    pub error: Option<String>,
    pub time_in: String,
    pub time_out: String,
    #[serde(default)]
    pub req_id: Option<u64>,
}

/// Builder for constructing add_order requests with validation.
#[derive(Debug, Clone)]
pub struct AddOrderBuilder {
    order_type: OrderType,
    side: OrderSide,
    symbol: String,
    order_qty: Decimal,
    limit_price: Option<Decimal>,
    time_in_force: Option<TimeInForce>,
    expire_time: Option<String>,
    post_only: Option<bool>,
    reduce_only: Option<bool>,
    margin: Option<bool>,
    cl_ord_id: Option<String>,
    order_userref: Option<i64>,
    validate: Option<bool>,
    triggers: Option<TriggerParams>,
    conditional: Option<ConditionalOrder>,
    display_qty: Option<Decimal>,
    stp_type: Option<StpType>,
    fee_preference: Option<FeeCurrencyPreference>,
    no_mpp: Option<bool>,
    req_id: Option<u64>,
}

impl AddOrderBuilder {
    /// Creates a new builder for a market order.
    #[must_use]
    pub fn market(side: OrderSide, symbol: &str, qty: Decimal) -> Self {
        Self::new(OrderType::Market, side, symbol, qty)
    }

    /// Creates a new builder for a limit order.
    #[must_use]
    pub fn limit(side: OrderSide, symbol: &str, qty: Decimal, price: Decimal) -> Self {
        let mut builder = Self::new(OrderType::Limit, side, symbol, qty);
        builder.limit_price = Some(price);
        builder
    }

    /// Creates a new builder for a stop-loss order.
    #[must_use]
    pub fn stop_loss(side: OrderSide, symbol: &str, qty: Decimal, trigger: TriggerParams) -> Self {
        let mut builder = Self::new(OrderType::StopLoss, side, symbol, qty);
        builder.triggers = Some(trigger);
        builder
    }

    /// Creates a new builder for a stop-loss limit order.
    #[must_use]
    pub fn stop_loss_limit(
        side: OrderSide,
        symbol: &str,
        qty: Decimal,
        price: Decimal,
        trigger: TriggerParams,
    ) -> Self {
        let mut builder = Self::new(OrderType::StopLossLimit, side, symbol, qty);
        builder.limit_price = Some(price);
        builder.triggers = Some(trigger);
        builder
    }

    /// Creates a new builder for a take-profit order.
    #[must_use]
    pub fn take_profit(
        side: OrderSide,
        symbol: &str,
        qty: Decimal,
        trigger: TriggerParams,
    ) -> Self {
        let mut builder = Self::new(OrderType::TakeProfit, side, symbol, qty);
        builder.triggers = Some(trigger);
        builder
    }

    /// Creates a new builder for a take-profit limit order.
    #[must_use]
    pub fn take_profit_limit(
        side: OrderSide,
        symbol: &str,
        qty: Decimal,
        price: Decimal,
        trigger: TriggerParams,
    ) -> Self {
        let mut builder = Self::new(OrderType::TakeProfitLimit, side, symbol, qty);
        builder.limit_price = Some(price);
        builder.triggers = Some(trigger);
        builder
    }

    /// Creates a new builder for an iceberg order.
    #[must_use]
    pub fn iceberg(
        side: OrderSide,
        symbol: &str,
        qty: Decimal,
        price: Decimal,
        display_qty: Decimal,
    ) -> Self {
        let mut builder = Self::new(OrderType::Iceberg, side, symbol, qty);
        builder.limit_price = Some(price);
        builder.display_qty = Some(display_qty);
        builder
    }

    fn new(order_type: OrderType, side: OrderSide, symbol: &str, qty: Decimal) -> Self {
        Self {
            order_type,
            side,
            symbol: symbol.to_string(),
            order_qty: qty,
            limit_price: None,
            time_in_force: None,
            expire_time: None,
            post_only: None,
            reduce_only: None,
            margin: None,
            cl_ord_id: None,
            order_userref: None,
            validate: None,
            triggers: None,
            conditional: None,
            display_qty: None,
            stp_type: None,
            fee_preference: None,
            no_mpp: None,
            req_id: None,
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

    /// Sets validate mode (dry-run without placing order).
    #[must_use]
    pub fn with_validate(mut self, validate: bool) -> Self {
        self.validate = Some(validate);
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

    /// Sets the no market price protection flag.
    #[must_use]
    pub fn with_no_mpp(mut self, no_mpp: bool) -> Self {
        self.no_mpp = Some(no_mpp);
        self
    }

    /// Sets the request ID for correlation.
    #[must_use]
    pub fn with_req_id(mut self, req_id: u64) -> Self {
        self.req_id = Some(req_id);
        self
    }

    /// Validates and builds the order parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing for the order type.
    pub fn build(self, token: &str) -> Result<AddOrderParams, AddOrderError> {
        self.validate()?;

        Ok(AddOrderParams {
            order_type: self.order_type,
            side: self.side,
            symbol: self.symbol,
            order_qty: self.order_qty,
            limit_price: self.limit_price,
            time_in_force: self.time_in_force,
            expire_time: self.expire_time,
            post_only: self.post_only,
            reduce_only: self.reduce_only,
            margin: self.margin,
            cl_ord_id: self.cl_ord_id,
            order_userref: self.order_userref,
            validate: self.validate,
            triggers: self.triggers,
            conditional: self.conditional,
            display_qty: self.display_qty,
            stp_type: self.stp_type,
            fee_preference: self.fee_preference,
            no_mpp: self.no_mpp,
            token: super::RedactedToken::new(token),
        })
    }

    /// Validates and builds the full request.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing for the order type.
    pub fn build_request(self, token: &str) -> Result<AddOrderRequest, AddOrderError> {
        let req_id = self.req_id;
        let params = self.build(token)?;
        Ok(AddOrderRequest::new(params, req_id))
    }

    fn validate(&self) -> Result<(), AddOrderError> {
        // Validate limit_price is required for limit-type orders
        if self.requires_limit_price() && self.limit_price.is_none() {
            return Err(AddOrderError::MissingLimitPrice(self.order_type));
        }

        // Validate triggers are required for stop/take-profit orders
        if self.requires_triggers() && self.triggers.is_none() {
            return Err(AddOrderError::MissingTriggers(self.order_type));
        }

        // Validate expire_time is required for GTD orders
        if self.time_in_force == Some(TimeInForce::Gtd) && self.expire_time.is_none() {
            return Err(AddOrderError::MissingExpireTime);
        }

        // Validate display_qty is only for iceberg orders
        if self.display_qty.is_some() && self.order_type != OrderType::Iceberg {
            return Err(AddOrderError::InvalidDisplayQty);
        }

        Ok(())
    }

    fn requires_limit_price(&self) -> bool {
        matches!(
            self.order_type,
            OrderType::Limit
                | OrderType::StopLossLimit
                | OrderType::TakeProfitLimit
                | OrderType::TrailingStopLimit
                | OrderType::Iceberg
        )
    }

    fn requires_triggers(&self) -> bool {
        matches!(
            self.order_type,
            OrderType::StopLoss
                | OrderType::StopLossLimit
                | OrderType::TakeProfit
                | OrderType::TakeProfitLimit
                | OrderType::TrailingStop
                | OrderType::TrailingStopLimit
        )
    }
}

/// Errors that can occur when building an add_order request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddOrderError {
    /// Limit price is required for this order type.
    MissingLimitPrice(OrderType),
    /// Trigger parameters are required for this order type.
    MissingTriggers(OrderType),
    /// Expire time is required for GTD orders.
    MissingExpireTime,
    /// Display quantity is only valid for iceberg orders.
    InvalidDisplayQty,
}

impl std::fmt::Display for AddOrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingLimitPrice(ot) => write!(f, "limit_price required for {:?} orders", ot),
            Self::MissingTriggers(ot) => write!(f, "triggers required for {:?} orders", ot),
            Self::MissingExpireTime => write!(f, "expire_time required for GTD orders"),
            Self::InvalidDisplayQty => write!(f, "display_qty only valid for iceberg orders"),
        }
    }
}

impl std::error::Error for AddOrderError {}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn serialize_market_order() {
        let params = AddOrderBuilder::market(OrderSide::Buy, "BTC/USD", dec!(0.001))
            .build("test_token")
            .unwrap();

        let request = AddOrderRequest::new(params, Some(42));
        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["method"], "add_order");
        assert_eq!(value["req_id"], 42);
        assert_eq!(value["params"]["order_type"], "market");
        assert_eq!(value["params"]["side"], "buy");
        assert_eq!(value["params"]["symbol"], "BTC/USD");
        assert_eq!(value["params"]["order_qty"], "0.001");
        assert_eq!(value["params"]["token"], "test_token");
    }

    #[test]
    fn serialize_limit_order() {
        let params = AddOrderBuilder::limit(OrderSide::Sell, "ETH/USD", dec!(1.5), dec!(2500))
            .with_time_in_force(TimeInForce::Gtc)
            .with_post_only(true)
            .build("test_token")
            .unwrap();

        let request = AddOrderRequest::new(params, None);
        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["params"]["order_type"], "limit");
        assert_eq!(value["params"]["limit_price"], "2500");
        assert_eq!(value["params"]["time_in_force"], "gtc");
        assert_eq!(value["params"]["post_only"], true);
        assert!(value.get("req_id").is_none());
    }

    #[test]
    fn serialize_stop_loss_order() {
        let trigger = TriggerParams::new(TriggerReference::Last, dec!(40000));
        let params = AddOrderBuilder::stop_loss(OrderSide::Sell, "BTC/USD", dec!(0.5), trigger)
            .build("test_token")
            .unwrap();

        let json = serde_json::to_string(&AddOrderRequest::new(params, None)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["params"]["order_type"], "stop-loss");
        assert_eq!(value["params"]["triggers"]["reference"], "last");
        assert_eq!(value["params"]["triggers"]["price"], "40000");
    }

    #[test]
    fn validate_limit_requires_price() {
        let result = AddOrderBuilder::new(OrderType::Limit, OrderSide::Buy, "BTC/USD", dec!(1.0))
            .build("token");

        assert!(matches!(
            result,
            Err(AddOrderError::MissingLimitPrice(OrderType::Limit))
        ));
    }

    #[test]
    fn validate_stop_loss_requires_triggers() {
        let result =
            AddOrderBuilder::new(OrderType::StopLoss, OrderSide::Sell, "BTC/USD", dec!(1.0))
                .build("token");

        assert!(matches!(
            result,
            Err(AddOrderError::MissingTriggers(OrderType::StopLoss))
        ));
    }

    #[test]
    fn validate_gtd_requires_expire_time() {
        let result = AddOrderBuilder::market(OrderSide::Buy, "BTC/USD", dec!(1.0))
            .with_time_in_force(TimeInForce::Gtd)
            .build("token");

        assert!(matches!(result, Err(AddOrderError::MissingExpireTime)));
    }

    #[test]
    fn validate_display_qty_only_for_iceberg() {
        let result = AddOrderBuilder::limit(OrderSide::Buy, "BTC/USD", dec!(1.0), dec!(50000))
            .with_display_qty(dec!(0.1))
            .build("token");

        assert!(matches!(result, Err(AddOrderError::InvalidDisplayQty)));
    }

    #[test]
    fn deserialize_success_response() {
        let json = r#"{
            "method": "add_order",
            "success": true,
            "result": {
                "order_id": "OXXXXXX-XXXXX-XXXXXX",
                "cl_ord_id": "my-order-1"
            },
            "time_in": "2024-01-15T12:00:00.000000Z",
            "time_out": "2024-01-15T12:00:00.001000Z",
            "req_id": 42
        }"#;

        let response: AddOrderResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert_eq!(
            response.result.as_ref().unwrap().order_id,
            "OXXXXXX-XXXXX-XXXXXX"
        );
        assert_eq!(
            response.result.as_ref().unwrap().cl_ord_id,
            Some("my-order-1".to_string())
        );
        assert_eq!(response.req_id, Some(42));
    }

    #[test]
    fn deserialize_error_response() {
        let json = r#"{
            "method": "add_order",
            "success": false,
            "error": "EOrder:Insufficient funds",
            "time_in": "2024-01-15T12:00:00.000000Z",
            "time_out": "2024-01-15T12:00:00.001000Z"
        }"#;

        let response: AddOrderResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(
            response.error,
            Some("EOrder:Insufficient funds".to_string())
        );
        assert!(response.result.is_none());
    }

    #[test]
    fn iceberg_order_with_display_qty() {
        let params =
            AddOrderBuilder::iceberg(OrderSide::Buy, "BTC/USD", dec!(10), dec!(50000), dec!(1))
                .build("token")
                .unwrap();

        let json = serde_json::to_string(&AddOrderRequest::new(params, None)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["params"]["order_type"], "iceberg");
        assert_eq!(value["params"]["display_qty"], "1");
    }
}
