use serde::Deserialize;

#[derive(Deserialize)]
pub struct TickerUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<TickerData>,
}

#[derive(Deserialize)]
pub struct TickerData {
    pub symbol: String,
    pub bid: f64,
    pub bid_qty: f64,
    pub ask: f64,
    pub ask_qty: f64,
    pub last: f64,
    pub volume: f64,
    pub vwap: f64,
    pub low: f64,
    pub high: f64,
    pub change: f64,
    pub change_pct: f64,
}
