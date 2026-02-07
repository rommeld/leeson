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
pub struct ChannelLimits {
    pub ticker: usize,
    pub book: usize,
    pub candle: usize,
    pub trade: usize,
    pub instrument: usize,
}

/// A `subscribe` request sent to the Kraken WebSocket API.
#[derive(Serialize)]
pub struct SubscribeRequest {
    pub method: String,
    pub params: Params,
}

/// An `unsubscribe` request sent to the Kraken WebSocket API.
#[derive(Serialize)]
pub struct UnsubscribeRequest {
    pub method: String,
    pub params: Params,
}

/// Channel and symbol parameters used in subscribe/unsubscribe requests.
#[derive(Serialize)]
pub struct Params {
    pub channel: String,
    pub symbol: Vec<String>,
}

/// A `ping` request used to test connection liveness.
#[derive(Serialize)]
pub struct PingRequest {
    pub method: String,
}

/// Server response to a [`PingRequest`].
#[derive(Deserialize)]
pub struct PongResponse {
    pub method: String,
    pub time_in: String,
    pub time_out: String,
}

/// Periodic heartbeat message indicating the connection is alive.
#[derive(Deserialize)]
pub struct HeartbeatResponse {
    pub channel: String,
}

/// System status update broadcast on the `status` channel.
#[derive(Deserialize)]
pub struct StatusUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<StatusData>,
}

/// Detailed system status information.
#[derive(Deserialize)]
pub struct StatusData {
    pub api_version: String,
    pub connection_id: u64,
    pub system: String,
    pub version: String,
}
