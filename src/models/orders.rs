//! Level-3 (individual orders) channel models.

use rust_decimal::Decimal;
use serde::Deserialize;

/// An update message from the `level3` (orders) channel.
#[derive(Debug, Clone, Deserialize)]
pub struct OrdersUpdateResponse {
    pub channel: String,
    /// Message type (e.g., `"snapshot"` or `"update"`).
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<OrdersData>,
}

/// Level-3 order book data for a single trading pair.
#[derive(Debug, Clone, Deserialize)]
pub struct OrdersData {
    pub symbol: String,
    /// Bid (buy) side individual orders.
    pub bids: Vec<OrderEntry>,
    /// Ask (sell) side individual orders.
    pub asks: Vec<OrderEntry>,
    /// Checksum for verifying order book integrity.
    pub checksum: u64,
    pub timestamp: String,
}

/// A single order in the level-3 book.
#[derive(Debug, Clone, Deserialize)]
pub struct OrderEntry {
    /// Lifecycle event for this order: `"add"`, `"modify"`, or `"delete"`.
    /// `None` on the initial snapshot.
    pub event: Option<String>,
    pub order_id: String,
    pub limit_price: Decimal,
    pub order_qty: Decimal,
    pub timestamp: String,
}
