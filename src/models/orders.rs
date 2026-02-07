//! Level-3 (individual orders) channel models.

use serde::Deserialize;

/// An update message from the `level3` (orders) channel.
#[derive(Deserialize)]
pub struct OrdersUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<OrdersData>,
}

/// Level-3 order book data for a single trading pair.
#[derive(Deserialize)]
pub struct OrdersData {
    pub symbol: String,
    pub bids: Vec<OrderEntry>,
    pub asks: Vec<OrderEntry>,
    pub checksum: u64,
    pub timestamp: String,
}

/// A single order in the level-3 book.
#[derive(Deserialize)]
pub struct OrderEntry {
    /// Lifecycle event for this order: `"add"`, `"modify"`, or `"delete"`.
    /// `None` on the initial snapshot.
    pub event: Option<String>,
    pub order_id: String,
    pub limit_price: f64,
    pub order_qty: f64,
    pub timestamp: String,
}
