use serde::Deserialize;

#[derive(Deserialize)]
pub struct BookUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<BookData>,
}

#[derive(Deserialize)]
pub struct BookData {
    pub symbol: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub checksum: u64,
    pub timestamp: String,
}

#[derive(Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub qty: f64,
}
