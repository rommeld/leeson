pub mod book;
pub mod candle;
pub mod instrument;
pub mod orders;
pub mod ticker;
pub mod trade;

use serde::{Deserialize, Serialize};

// --- Channel ---

pub enum Channel {
    Book,
    Ticker,
    Orders,
    Candles,
    Trades,
    Instruments,
    Status,
    Heartbeat,
}

impl Channel {
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

// --- Channel limits ---

pub struct ChannelLimits {
    pub ticker: usize,
    pub book: usize,
    pub candle: usize,
    pub trade: usize,
    pub instrument: usize,
}

// --- Subscribe / Unsubscribe ---

#[derive(Serialize)]
pub struct SubscribeRequest {
    pub method: String,
    pub params: Params,
}

#[derive(Serialize)]
pub struct UnsubscribeRequest {
    pub method: String,
    pub params: Params,
}

#[derive(Serialize)]
pub struct Params {
    pub channel: String,
    pub symbol: Vec<String>,
}

// --- Ping / Pong ---

#[derive(Serialize)]
pub struct PingRequest {
    pub method: String,
}

#[derive(Deserialize)]
pub struct PongResponse {
    pub method: String,
    pub time_in: String,
    pub time_out: String,
}

// --- Heartbeat ---

#[derive(Deserialize)]
pub struct HeartbeatResponse {
    pub channel: String,
}

// --- Status ---

#[derive(Deserialize)]
pub struct StatusUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<StatusData>,
}

#[derive(Deserialize)]
pub struct StatusData {
    pub api_version: String,
    pub connection_id: u64,
    pub system: String,
    pub version: String,
}
