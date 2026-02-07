//! Ticker (price summary) channel models.

use serde::Deserialize;

/// An update message from the `ticker` channel.
#[derive(Deserialize)]
pub struct TickerUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<TickerData>,
}

/// Real-time ticker snapshot for a single trading pair.
#[derive(Deserialize)]
pub struct TickerData {
    pub symbol: String,
    pub bid: f64,
    /// Quantity available at the best bid price.
    pub bid_qty: f64,
    pub ask: f64,
    /// Quantity available at the best ask price.
    pub ask_qty: f64,
    pub last: f64,
    pub volume: f64,
    /// Volume-weighted average price.
    pub vwap: f64,
    pub low: f64,
    pub high: f64,
    /// Absolute price change over the last 24 hours.
    pub change: f64,
    /// Price change as a percentage over the last 24 hours.
    pub change_pct: f64,
}
