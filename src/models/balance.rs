//! Balance channel models.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Response from the balances channel (snapshot or update).
#[derive(Debug, Clone, Deserialize)]
pub struct BalanceResponse {
    pub channel: String,
    /// Message type: `"snapshot"` or `"update"`.
    #[serde(rename = "type")]
    pub tpe: String,
    pub data: Vec<BalanceData>,
    #[serde(default)]
    pub sequence: u64,
}

/// Balance data for a single asset (used in snapshots).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct BalanceData {
    /// Asset symbol (e.g., "BTC", "USD").
    pub asset: String,
    /// Asset class (e.g., "currency").
    #[serde(default)]
    pub asset_class: String,
    /// Total balance across all wallets.
    pub balance: Decimal,
    /// Breakdown by wallet type.
    #[serde(default)]
    pub wallets: Vec<WalletBalance>,
}

/// Balance for a specific wallet.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct WalletBalance {
    /// Wallet type: "spot" or "earn".
    #[serde(rename = "type")]
    pub wallet_type: String,
    /// Wallet identifier (e.g., "main", "flex", "bonded").
    #[serde(default)]
    pub id: String,
    /// Balance in this wallet.
    pub balance: Decimal,
}

/// Balance update data (used in updates after transactions).
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen, get_all, from_py_object))]
pub struct BalanceUpdateData {
    /// Ledger entry identifier.
    pub ledger_id: String,
    /// Reference ID (e.g., trade_id).
    pub ref_id: String,
    /// RFC3339 timestamp.
    pub timestamp: String,
    /// Event type: deposit, withdrawal, trade, margin, adjustment, etc.
    #[serde(rename = "type")]
    pub update_type: String,
    /// Asset symbol.
    pub asset: String,
    /// Change amount.
    pub amount: Decimal,
    /// Current total balance after this change.
    pub balance: Decimal,
    /// Transaction fee.
    #[serde(default)]
    pub fee: Decimal,
    /// Wallet type: "spot" or "earn".
    pub wallet_type: String,
    /// Wallet identifier.
    pub wallet_id: String,
}
