//! Shared models for Kraken WebSocket V2 messages.
//!
//! Contains channel definitions, subscription request/response types,
//! and common protocol messages (ping/pong, heartbeat, status).

pub mod add_order;
pub mod amend_order;
pub mod batch_add;
pub mod batch_cancel;
pub mod book;
pub mod cancel_after;
pub mod cancel_all;
pub mod cancel_order;
pub mod candle;
pub mod edit_order;
pub mod execution;
pub mod instrument;
pub mod orders;
pub mod ticker;
pub mod trade;

pub use add_order::{
    AddOrderBuilder, AddOrderError, AddOrderParams, AddOrderRequest, AddOrderResponse,
    AddOrderResult, ConditionalOrder, FeeCurrencyPreference, OrderSide, OrderType, StpType,
    TimeInForce, TriggerParams, TriggerPriceType, TriggerReference,
};
pub use amend_order::{
    AmendOrderBuilder, AmendOrderError, AmendOrderParams, AmendOrderRequest, AmendOrderResponse,
    AmendOrderResult, PriceType,
};
pub use batch_add::{
    BatchAddBuilder, BatchAddError, BatchAddOrderResult, BatchAddRequest, BatchAddResponse,
    BatchOrderEntry, MAX_BATCH_SIZE, MIN_BATCH_SIZE,
};
pub use batch_cancel::{
    BatchCancelBuilder, BatchCancelError, BatchCancelRequest, BatchCancelResponse,
    BatchCancelResult, MAX_BATCH_CANCEL_SIZE, MIN_BATCH_CANCEL_SIZE,
};
pub use cancel_after::{
    CancelAfterRequest, CancelAfterResponse, CancelAfterResult, MAX_TIMEOUT_SECONDS,
};
pub use cancel_all::{CancelAllRequest, CancelAllResponse, CancelAllResult};
pub use cancel_order::{
    CancelOrderBuilder, CancelOrderError, CancelOrderParams, CancelOrderRequest,
    CancelOrderResponse, CancelOrderResult,
};
pub use edit_order::{
    EditOrderBuilder, EditOrderError, EditOrderParams, EditOrderRequest, EditOrderResponse,
    EditOrderResult,
};

use std::fmt;

use serde::{Deserialize, Serialize};

/// A `String` wrapper whose [`Debug`] output replaces the value with
/// `[REDACTED]` so authentication tokens are never leaked into logs.
///
/// It serializes transparently as a plain string and dereferences to
/// `&str` / `&String` for ergonomic use.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct RedactedToken(String);

impl RedactedToken {
    /// Wraps a token string.
    #[must_use]
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// Returns the inner token value.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Debug for RedactedToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl std::ops::Deref for RedactedToken {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for RedactedToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// Available Kraken WebSocket V2 channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    Book,
    Ticker,
    /// Level-3 individual orders (wire name: `"level3"`).
    Orders,
    /// OHLC candlestick data (wire name: `"ohlc"`).
    Candles,
    Trades,
    Instruments,
    /// User execution reports (wire name: `"executions"`).
    Executions,
    Status,
    Heartbeat,
}

impl Channel {
    /// Returns the wire-format channel name expected by the Kraken API.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Book => "book",
            Channel::Ticker => "ticker",
            Channel::Orders => "level3",
            Channel::Candles => "ohlc",
            Channel::Trades => "trade",
            Channel::Instruments => "instrument",
            Channel::Executions => "executions",
            Channel::Status => "status",
            Channel::Heartbeat => "heartbeat",
        }
    }
}

/// A `subscribe` request sent to the Kraken WebSocket API.
#[derive(Debug, Clone, Serialize)]
pub struct SubscribeRequest {
    method: String,
    params: Params,
}

impl SubscribeRequest {
    /// Creates a new subscribe request for the given channel, symbols, and optional auth token.
    #[must_use]
    pub fn new(channel: &Channel, symbols: &[String], token: Option<String>) -> Self {
        Self {
            method: "subscribe".to_string(),
            params: Params::new(channel, symbols, token),
        }
    }
}

/// An `unsubscribe` request sent to the Kraken WebSocket API.
#[derive(Debug, Clone, Serialize)]
pub struct UnsubscribeRequest {
    method: String,
    params: Params,
}

impl UnsubscribeRequest {
    /// Creates a new unsubscribe request for the given channel, symbols, and optional auth token.
    #[must_use]
    pub fn new(channel: &Channel, symbols: &[String], token: Option<String>) -> Self {
        Self {
            method: "unsubscribe".to_string(),
            params: Params::new(channel, symbols, token),
        }
    }
}

/// Channel and symbol parameters used in subscribe/unsubscribe requests.
#[derive(Debug, Clone, Serialize)]
pub struct Params {
    channel: String,
    symbol: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<RedactedToken>,
}

impl Params {
    /// Creates new parameters for the given channel, symbols, and optional auth token.
    #[must_use]
    pub fn new(channel: &Channel, symbols: &[String], token: Option<String>) -> Self {
        Self {
            channel: channel.as_str().to_string(),
            symbol: symbols.to_vec(),
            token: token.map(RedactedToken::new),
        }
    }
}

/// Parameters for book channel subscription with depth option.
#[derive(Debug, Clone, Serialize)]
pub struct BookParams {
    channel: String,
    symbol: Vec<String>,
    depth: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<RedactedToken>,
}

impl BookParams {
    /// Creates new book parameters for the given symbols and depth.
    #[must_use]
    pub fn new(symbols: &[String], depth: book::BookDepth, token: Option<String>) -> Self {
        Self {
            channel: Channel::Book.as_str().to_string(),
            symbol: symbols.to_vec(),
            depth: depth.as_u16(),
            token: token.map(RedactedToken::new),
        }
    }
}

/// A `subscribe` request for the book channel with depth parameter.
#[derive(Debug, Clone, Serialize)]
pub struct BookSubscribeRequest {
    method: String,
    params: BookParams,
}

impl BookSubscribeRequest {
    /// Creates a new book subscribe request with the specified depth.
    #[must_use]
    pub fn new(symbols: &[String], depth: book::BookDepth, token: Option<String>) -> Self {
        Self {
            method: "subscribe".to_string(),
            params: BookParams::new(symbols, depth, token),
        }
    }
}

/// An `unsubscribe` request for the book channel with depth parameter.
#[derive(Debug, Clone, Serialize)]
pub struct BookUnsubscribeRequest {
    method: String,
    params: BookParams,
}

impl BookUnsubscribeRequest {
    /// Creates a new book unsubscribe request with the specified depth.
    #[must_use]
    pub fn new(symbols: &[String], depth: book::BookDepth) -> Self {
        Self {
            method: "unsubscribe".to_string(),
            params: BookParams::new(symbols, depth, None),
        }
    }
}

/// Parameters for executions channel subscription.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionsParams {
    channel: String,
    token: RedactedToken,
    snap_orders: bool,
    snap_trades: bool,
}

impl ExecutionsParams {
    /// Creates new executions parameters.
    #[must_use]
    pub fn new(token: &str, snap_orders: bool, snap_trades: bool) -> Self {
        Self {
            channel: Channel::Executions.as_str().to_string(),
            token: RedactedToken::new(token),
            snap_orders,
            snap_trades,
        }
    }
}

/// A `subscribe` request for the executions channel.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionsSubscribeRequest {
    method: String,
    params: ExecutionsParams,
}

impl ExecutionsSubscribeRequest {
    /// Creates a new executions subscribe request.
    #[must_use]
    pub fn new(token: &str, snap_orders: bool, snap_trades: bool) -> Self {
        Self {
            method: "subscribe".to_string(),
            params: ExecutionsParams::new(token, snap_orders, snap_trades),
        }
    }
}

/// An `unsubscribe` request for the executions channel.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionsUnsubscribeRequest {
    method: String,
    params: ExecutionsUnsubscribeParams,
}

/// Parameters for executions channel unsubscription.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionsUnsubscribeParams {
    channel: String,
    token: RedactedToken,
}

impl ExecutionsUnsubscribeRequest {
    /// Creates a new executions unsubscribe request.
    #[must_use]
    pub fn new(token: &str) -> Self {
        Self {
            method: "unsubscribe".to_string(),
            params: ExecutionsUnsubscribeParams {
                channel: Channel::Executions.as_str().to_string(),
                token: RedactedToken::new(token),
            },
        }
    }
}

/// A `ping` request used to test connection liveness.
#[derive(Debug, Clone, Serialize)]
pub struct PingRequest {
    method: String,
}

impl PingRequest {
    /// Creates a new ping request.
    #[must_use]
    pub fn new() -> Self {
        Self {
            method: "ping".to_string(),
        }
    }
}

impl Default for PingRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Server response to a [`PingRequest`].
#[derive(Debug, Clone, Deserialize)]
pub struct PongResponse {
    pub method: String,
    pub time_in: String,
    pub time_out: String,
}

/// Periodic heartbeat message indicating the connection is alive.
#[derive(Debug, Clone, Deserialize)]
pub struct HeartbeatResponse {
    pub channel: String,
}

/// System status update broadcast on the `status` channel.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusUpdateResponse {
    pub channel: String,
    /// Message type (e.g., `"snapshot"` or `"update"`).
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<StatusData>,
}

/// Detailed system status information.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusData {
    /// API version (e.g., `"v2"`).
    pub api_version: String,
    /// Unique identifier for this WebSocket connection.
    pub connection_id: u64,
    /// System status (e.g., `"online"`, `"maintenance"`).
    pub system: String,
    /// Server software version.
    pub version: String,
}
