use serde::Deserialize;

#[derive(Deserialize)]
pub struct OrdersUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<OrdersData>,
}

#[derive(Deserialize)]
pub struct OrdersData {
    pub symbol: String,
    pub bids: Vec<OrderEntry>,
    pub asks: Vec<OrderEntry>,
    pub checksum: u64,
    pub timestamp: String,
}

#[derive(Deserialize)]
pub struct OrderEntry {
    pub event: Option<String>,
    pub order_id: String,
    pub limit_price: f64,
    pub order_qty: f64,
    pub timestamp: String,
}
