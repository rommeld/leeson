use serde::Deserialize;

#[derive(Deserialize)]
pub struct CandleUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub timestamp: String,
    pub data: Vec<CandleData>,
}

#[derive(Deserialize)]
pub struct CandleData {
    pub symbol: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub vwap: f64,
    pub trades: u64,
    pub volume: f64,
    pub interval_begin: String,
    pub interval: u64,
    pub timestamp: String,
}
