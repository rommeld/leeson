//! OHLC candlestick channel models.

use rust_decimal::Decimal;
use serde::Deserialize;

/// An update message from the `ohlc` (candles) channel.
#[derive(Debug, Clone, Deserialize)]
pub struct CandleUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub timestamp: String,
    pub data: Vec<CandleData>,
}

/// A single OHLC candlestick bar.
#[derive(Debug, Clone, Deserialize)]
pub struct CandleData {
    pub symbol: String,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    /// Volume-weighted average price for this candle.
    pub vwap: Decimal,
    pub trades: u64,
    pub volume: Decimal,
    /// Start timestamp of this candle's time window.
    pub interval_begin: String,
    /// Candle duration in minutes.
    pub interval: u64,
    pub timestamp: String,
}
