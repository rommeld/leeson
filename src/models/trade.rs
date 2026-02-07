use serde::Deserialize;

#[derive(Deserialize)]
pub struct TradeUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<TradeData>,
}

#[derive(Deserialize)]
pub struct TradeData {
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub qty: f64,
    pub ord_type: String,
    pub trade_id: u64,
    pub timestamp: String,
}
