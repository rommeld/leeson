//! Ticker (price summary) channel models.

use rust_decimal::Decimal;
use serde::Deserialize;

/// An update message from the `ticker` channel.
#[derive(Debug, Clone, Deserialize)]
pub struct TickerUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<TickerData>,
}

/// Real-time ticker snapshot for a single trading pair.
#[derive(Debug, Clone, Deserialize)]
pub struct TickerData {
    pub symbol: String,
    pub bid: Decimal,
    /// Quantity available at the best bid price.
    pub bid_qty: Decimal,
    pub ask: Decimal,
    /// Quantity available at the best ask price.
    pub ask_qty: Decimal,
    pub last: Decimal,
    pub volume: Decimal,
    /// Volume-weighted average price.
    pub vwap: Decimal,
    pub low: Decimal,
    pub high: Decimal,
    /// Absolute price change over the last 24 hours.
    pub change: Decimal,
    /// Price change as a percentage over the last 24 hours.
    pub change_pct: Decimal,
}
