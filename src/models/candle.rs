//! OHLC candlestick channel models.

use serde::Deserialize;

/// An update message from the `ohlc` (candles) channel.
#[derive(Deserialize)]
pub struct CandleUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub timestamp: String,
    pub data: Vec<CandleData>,
}

/// A single OHLC candlestick bar.
#[derive(Deserialize)]
pub struct CandleData {
    pub symbol: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    /// Volume-weighted average price for this candle.
    pub vwap: f64,
    pub trades: u64,
    pub volume: f64,
    /// Start timestamp of this candle's time window.
    pub interval_begin: String,
    /// Candle duration in minutes.
    pub interval: u64,
    pub timestamp: String,
}
