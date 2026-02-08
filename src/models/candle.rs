//! OHLC candlestick channel models.

use rust_decimal::Decimal;
use serde::Deserialize;

/// An update message from the `ohlc` (candles) channel.
#[derive(Debug, Clone, Deserialize)]
pub struct CandleUpdateResponse {
    pub channel: String,
    /// Message type (e.g., `"snapshot"` or `"update"`).
    #[serde(rename = "type")]
    pub tpe: String,
    /// Server-side timestamp of this message.
    pub timestamp: String,
    pub data: Vec<CandleData>,
}

/// A single OHLC candlestick bar.
#[derive(Debug, Clone, Deserialize)]
pub struct CandleData {
    pub symbol: String,
    /// Opening price of the candle.
    pub open: Decimal,
    /// Highest price during the candle.
    pub high: Decimal,
    /// Lowest price during the candle.
    pub low: Decimal,
    /// Closing (most recent) price of the candle.
    pub close: Decimal,
    /// Volume-weighted average price for this candle.
    pub vwap: Decimal,
    /// Number of trades during this candle.
    pub trades: u64,
    /// Total trade volume during this candle.
    pub volume: Decimal,
    /// Start timestamp of this candle's time window.
    pub interval_begin: String,
    /// Candle duration in minutes.
    pub interval: u64,
    /// End timestamp of this candle.
    pub timestamp: String,
}
