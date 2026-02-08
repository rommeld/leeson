//! Trade channel models.

use rust_decimal::Decimal;
use serde::Deserialize;

/// An update message from the `trade` channel.
#[derive(Debug, Clone, Deserialize)]
pub struct TradeUpdateResponse {
    pub channel: String,
    /// Message type (e.g., `"snapshot"` or `"update"`).
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<TradeData>,
}

/// A single executed trade.
#[derive(Debug, Clone, Deserialize)]
pub struct TradeData {
    pub symbol: String,
    /// Trade direction: `"buy"` or `"sell"`.
    pub side: String,
    /// Execution price.
    pub price: Decimal,
    /// Executed quantity.
    pub qty: Decimal,
    /// Order type that triggered this trade (e.g., `"market"`, `"limit"`).
    pub ord_type: String,
    /// Unique trade identifier.
    pub trade_id: u64,
    /// Execution timestamp.
    pub timestamp: String,
}
