//! Executions channel models.
//!
//! Streams order status changes and trade execution events for the
//! authenticated user's account.

use rust_decimal::Decimal;
use serde::Deserialize;

/// A message from the `executions` channel (snapshot or update).
#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionUpdateResponse {
    pub channel: String,
    /// Message type (e.g., `"snapshot"` or `"update"`).
    #[serde(rename = "type")]
    pub tpe: String,
    /// Monotonically increasing sequence number for ordering.
    pub sequence: i64,
    pub data: Vec<ExecutionData>,
}

/// A single execution report.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct ExecutionData {
    // -- Identifiers --
    pub order_id: String,
    pub order_userref: Option<i64>,
    pub cl_ord_id: Option<String>,
    pub exec_id: Option<String>,
    pub trade_id: Option<i64>,
    pub ord_ref_id: Option<String>,

    // -- Order details --
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub order_qty: Decimal,
    pub order_status: String,
    pub time_in_force: Option<String>,

    // -- Pricing --
    pub limit_price: Option<Decimal>,
    pub limit_price_type: Option<String>,
    pub avg_price: Option<Decimal>,
    pub last_price: Option<Decimal>,
    pub cash_order_qty: Option<Decimal>,

    // -- Execution --
    pub exec_type: String,
    pub last_qty: Option<Decimal>,
    pub cum_qty: Option<Decimal>,
    pub cum_cost: Option<Decimal>,
    pub cost: Option<Decimal>,
    pub liquidity_ind: Option<String>,

    // -- Fees --
    pub fees: Option<Vec<Fee>>,
    pub fee_ccy_pref: Option<String>,
    pub fee_usd_equiv: Option<Decimal>,

    // -- Timestamps --
    pub timestamp: String,
    pub effective_time: Option<String>,
    pub expire_time: Option<String>,

    // -- Flags --
    pub post_only: Option<bool>,
    pub reduce_only: Option<bool>,
    pub no_mpp: Option<bool>,
    pub margin: Option<bool>,
    pub margin_borrow: Option<bool>,
    pub amended: Option<bool>,
    pub liquidated: Option<bool>,

    // -- Display (iceberg) --
    pub display_qty: Option<Decimal>,
    pub display_qty_remain: Option<Decimal>,

    // -- Triggers --
    pub triggers: Option<Triggers>,

    // -- Contingent orders --
    pub contingent: Option<Contingent>,

    // -- Meta --
    pub reason: Option<String>,
    pub position_status: Option<String>,
    pub sender_sub_id: Option<String>,
}

/// Fee charged on a trade event.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct Fee {
    pub asset: String,
    pub qty: Decimal,
}

/// Trigger parameters for stop/trailing orders.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct Triggers {
    pub reference: Option<String>,
    pub price: Option<Decimal>,
    pub price_type: Option<String>,
    pub actual_price: Option<Decimal>,
    pub peak_price: Option<Decimal>,
    pub last_price: Option<Decimal>,
    pub status: Option<String>,
    pub timestamp: Option<String>,
}

/// Contingent (secondary) order template.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct Contingent {
    pub order_type: Option<String>,
    pub trigger_price: Option<Decimal>,
    pub trigger_price_type: Option<String>,
    pub limit_price: Option<Decimal>,
    pub limit_price_type: Option<String>,
}
