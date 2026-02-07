//! Instrument (asset and pair metadata) channel models.

use rust_decimal::Decimal;
use serde::Deserialize;

/// An update message from the `instrument` channel.
#[derive(Debug, Clone, Deserialize)]
pub struct InstrumentUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: InstrumentData,
}

/// Container for asset and trading pair reference data.
#[derive(Debug, Clone, Deserialize)]
pub struct InstrumentData {
    pub assets: Vec<AssetInfo>,
    pub pairs: Vec<PairInfo>,
}

/// Reference data for a single asset (currency).
#[derive(Debug, Clone, Deserialize)]
pub struct AssetInfo {
    pub id: String,
    pub status: String,
    pub precision: u32,
    /// Number of decimal places shown in the UI.
    pub precision_display: u32,
    pub borrowable: bool,
    /// Multiplier applied when using this asset as collateral (0.0–1.0).
    pub collateral_value: Decimal,
    /// Interest rate charged for margin borrowing.
    pub margin_rate: Decimal,
}

/// Reference data for a single trading pair.
#[derive(Debug, Clone, Deserialize)]
pub struct PairInfo {
    pub symbol: String,
    pub base: String,
    pub quote: String,
    pub status: String,
    pub qty_precision: u32,
    /// Minimum order quantity step size.
    pub qty_increment: Decimal,
    pub price_precision: u32,
    /// Minimum price tick size.
    pub price_increment: Decimal,
    pub cost_precision: u32,
    /// Minimum order cost in quote currency.
    pub cost_min: Decimal,
    pub qty_min: Decimal,
    pub marginable: bool,
    /// Required initial margin fraction for opening a position.
    pub margin_initial: Option<Decimal>,
    /// Maximum allowed long position size.
    pub position_limit_long: Option<u64>,
    /// Maximum allowed short position size.
    pub position_limit_short: Option<u64>,
    /// Whether a price index is available for this pair.
    pub has_index: bool,
}
