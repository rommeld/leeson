//! Order book channel models.

use rust_decimal::Decimal;
use serde::Deserialize;

/// An update message from the `book` channel.
#[derive(Debug, Clone, Deserialize)]
pub struct BookUpdateResponse {
    pub channel: String,
    /// Message type (e.g., `"snapshot"` or `"update"`).
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<BookData>,
}

/// Order book snapshot or incremental update for a single trading pair.
#[derive(Debug, Clone, Deserialize)]
pub struct BookData {
    pub symbol: String,
    /// Bid (buy) side price levels, sorted highest to lowest.
    pub bids: Vec<PriceLevel>,
    /// Ask (sell) side price levels, sorted lowest to highest.
    pub asks: Vec<PriceLevel>,
    /// CRC32 checksum used to verify order book integrity.
    pub checksum: u32,
    pub timestamp: String,
}

/// A single price level in the order book.
#[derive(Debug, Clone, Deserialize)]
pub struct PriceLevel {
    pub price: Decimal,
    pub qty: Decimal,
}

/// Available depth levels for order book subscriptions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum BookDepth {
    /// Top 10 price levels (default).
    #[default]
    D10,
    /// Top 25 price levels.
    D25,
    /// Top 100 price levels.
    D100,
    /// Top 500 price levels.
    D500,
    /// Top 1000 price levels.
    D1000,
}

impl BookDepth {
    /// Returns the numeric depth value for the Kraken API.
    #[must_use]
    pub fn as_u16(&self) -> u16 {
        match self {
            BookDepth::D10 => 10,
            BookDepth::D25 => 25,
            BookDepth::D100 => 100,
            BookDepth::D500 => 500,
            BookDepth::D1000 => 1000,
        }
    }
}

/// Formats a price level for CRC32 checksum calculation.
///
/// Removes decimal points and leading zeros from both price and quantity,
/// then concatenates them.
fn format_level_for_checksum(level: &PriceLevel) -> String {
    let price_str = level.price.to_string().replace('.', "");
    let qty_str = level.qty.to_string().replace('.', "");

    let price_trimmed = price_str.trim_start_matches('0');
    let qty_trimmed = qty_str.trim_start_matches('0');

    format!("{}{}", price_trimmed, qty_trimmed)
}

/// Calculates the CRC32 checksum for an order book.
///
/// The checksum is computed by:
/// 1. Taking the top 10 ask levels (sorted lowest to highest price)
/// 2. Taking the top 10 bid levels (sorted highest to lowest price)
/// 3. Formatting each level by removing decimals and leading zeros
/// 4. Concatenating asks string + bids string
/// 5. Computing CRC32 on the resulting string
///
/// Note: The checksum always uses the top 10 levels regardless of subscription depth.
#[must_use]
pub fn calculate_checksum(asks: &[PriceLevel], bids: &[PriceLevel]) -> u32 {
    let mut checksum_str = String::new();

    // Process asks (lowest to highest price) - take top 10
    for level in asks.iter().take(10) {
        checksum_str.push_str(&format_level_for_checksum(level));
    }

    // Process bids (highest to lowest price) - take top 10
    for level in bids.iter().take(10) {
        checksum_str.push_str(&format_level_for_checksum(level));
    }

    crc32fast::hash(checksum_str.as_bytes())
}

/// Verifies that the order book checksum matches the expected value.
#[must_use]
pub fn verify_checksum(book: &BookData) -> bool {
    calculate_checksum(&book.asks, &book.bids) == book.checksum
}
