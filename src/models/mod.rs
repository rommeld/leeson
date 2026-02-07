//! Shared models for Kraken WebSocket V2 messages.
//!
//! Contains channel definitions, subscription request/response types,
//! and common protocol messages (ping/pong, heartbeat, status).

pub mod book;
pub mod candle;
pub mod instrument;
pub mod orders;
pub mod ticker;
pub mod trade;

use serde::{Deserialize, Serialize};

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
            Channel::Status => "status",
            Channel::Heartbeat => "heartbeat",
        }
    }
}

/// Per-channel message limits controlling how many updates to collect
/// before automatically unsubscribing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelLimits {
    pub ticker: usize,
    pub book: usize,
    pub candle: usize,
    pub trade: usize,
    pub instrument: usize,
}

impl ChannelLimits {
    /// Creates a new `ChannelLimits` with the specified limits for each channel.
    #[must_use]
    pub fn new(ticker: usize, book: usize, candle: usize, trade: usize, instrument: usize) -> Self {
        Self {
            ticker,
            book,
            candle,
            trade,
            instrument,
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
    /// Creates a new subscribe request for the given channel and symbols.
    #[must_use]
    pub fn new(channel: &Channel, symbols: &[String]) -> Self {
        Self {
            method: "subscribe".to_string(),
            params: Params::new(channel, symbols),
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
    /// Creates a new unsubscribe request for the given channel and symbols.
    #[must_use]
    pub fn new(channel: &Channel, symbols: &[String]) -> Self {
        Self {
            method: "unsubscribe".to_string(),
            params: Params::new(channel, symbols),
        }
    }
}

/// Channel and symbol parameters used in subscribe/unsubscribe requests.
#[derive(Debug, Clone, Serialize)]
pub struct Params {
    channel: String,
    symbol: Vec<String>,
}

impl Params {
    /// Creates new parameters for the given channel and symbols.
    #[must_use]
    pub fn new(channel: &Channel, symbols: &[String]) -> Self {
        Self {
            channel: channel.as_str().to_string(),
            symbol: symbols.to_vec(),
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
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<StatusData>,
}

/// Detailed system status information.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusData {
    pub api_version: String,
    pub connection_id: u64,
    pub system: String,
    pub version: String,
}
