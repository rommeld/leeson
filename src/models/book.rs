//! Order book channel models.

use rust_decimal::Decimal;
use serde::Deserialize;

/// An update message from the `book` channel.
#[derive(Debug, Clone, Deserialize)]
pub struct BookUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<BookData>,
}

/// Order book snapshot or incremental update for a single trading pair.
#[derive(Debug, Clone, Deserialize)]
pub struct BookData {
    pub symbol: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    /// CRC32 checksum used to verify order book integrity.
    pub checksum: u64,
    pub timestamp: String,
}

/// A single price level in the order book.
#[derive(Debug, Clone, Deserialize)]
pub struct PriceLevel {
    pub price: Decimal,
    pub qty: Decimal,
}
