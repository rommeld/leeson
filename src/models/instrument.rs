use serde::Deserialize;

#[derive(Deserialize)]
pub struct InstrumentUpdateResponse {
    pub channel: String,
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: InstrumentData,
}

#[derive(Deserialize)]
pub struct InstrumentData {
    pub assets: Vec<AssetInfo>,
    pub pairs: Vec<PairInfo>,
}

#[derive(Deserialize)]
pub struct AssetInfo {
    pub id: String,
    pub status: String,
    pub precision: u32,
    pub precision_display: u32,
    pub borrowable: bool,
    pub collateral_value: f64,
    pub margin_rate: f64,
}

#[derive(Deserialize)]
pub struct PairInfo {
    pub symbol: String,
    pub base: String,
    pub quote: String,
    pub status: String,
    pub qty_precision: u32,
    pub qty_increment: f64,
    pub price_precision: u32,
    pub price_increment: f64,
    pub cost_precision: u32,
    pub cost_min: String,
    pub qty_min: f64,
    pub marginable: bool,
    pub margin_initial: Option<f64>,
    pub position_limit_long: Option<u64>,
    pub position_limit_short: Option<u64>,
    pub has_index: bool,
}
