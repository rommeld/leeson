//! Ticker (price summary) channel models.

use rust_decimal::Decimal;
use serde::Deserialize;

/// An update message from the `ticker` channel.
#[derive(Debug, Clone, Deserialize)]
pub struct TickerUpdateResponse {
    pub channel: String,
    /// Message type (e.g., `"snapshot"` or `"update"`).
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<TickerData>,
}

/// Real-time ticker snapshot for a single trading pair.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct TickerData {
    /// Trading pair symbol (e.g., `"BTC/USD"`).
    pub symbol: String,
    /// Best bid price.
    pub bid: Decimal,
    /// Quantity available at the best bid price.
    pub bid_qty: Decimal,
    /// Best ask price.
    pub ask: Decimal,
    /// Quantity available at the best ask price.
    pub ask_qty: Decimal,
    /// Last traded price.
    pub last: Decimal,
    /// 24-hour rolling trade volume.
    pub volume: Decimal,
    /// Volume-weighted average price.
    pub vwap: Decimal,
    /// 24-hour rolling low price.
    pub low: Decimal,
    /// 24-hour rolling high price.
    pub high: Decimal,
    /// Absolute price change over the last 24 hours.
    pub change: Decimal,
    /// Price change as a percentage over the last 24 hours.
    pub change_pct: Decimal,
}
