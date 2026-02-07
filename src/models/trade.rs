//! Trade channel models.

use serde::Deserialize;

/// An update message from the `trade` channel.
#[derive(Deserialize)]
pub struct TradeUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<TradeData>,
}

/// A single executed trade.
#[derive(Deserialize)]
pub struct TradeData {
    pub symbol: String,
    /// Trade direction: `"buy"` or `"sell"`.
    pub side: String,
    pub price: f64,
    pub qty: f64,
    /// Order type that triggered this trade (e.g., `"market"`, `"limit"`).
    pub ord_type: String,
    pub trade_id: u64,
    pub timestamp: String,
}
