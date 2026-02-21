//! Risk configuration types and loading.

use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

use rust_decimal::Decimal;
use serde::Deserialize;

/// Risk limits configuration loaded from `risk.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct RiskConfig {
    /// Default limits applied to all symbols unless overridden.
    pub defaults: SymbolLimits,
    /// Per-symbol overrides. Missing fields inherit from `defaults`.
    #[serde(default)]
    pub symbols: HashMap<String, SymbolOverrides>,
}

/// Complete set of limits (used as global defaults). All fields required.
#[derive(Debug, Clone, Deserialize)]
pub struct SymbolLimits {
    pub max_order_qty: Decimal,
    pub max_notional_value: Decimal,
    pub confirm_above_notional: Decimal,
    pub max_trades_per_day: u32,
    pub max_trades_per_week: u32,
    pub max_trades_per_month: u32,
}

/// Per-symbol overrides. Every field optional; missing inherits from defaults.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SymbolOverrides {
    pub max_order_qty: Option<Decimal>,
    pub max_notional_value: Option<Decimal>,
    pub confirm_above_notional: Option<Decimal>,
    pub max_trades_per_day: Option<u32>,
    pub max_trades_per_week: Option<u32>,
    pub max_trades_per_month: Option<u32>,
}

impl RiskConfig {
    /// Loads risk configuration from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> crate::Result<Self> {
        let contents = std::fs::read_to_string(path).map_err(|e| {
            crate::LeesonError::Config(format!("failed to read {}: {e}", path.display()))
        })?;
        let config: Self = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Returns the effective limits for a symbol, merging overrides with defaults.
    pub fn limits_for(&self, symbol: &str) -> SymbolLimits {
        match self.symbols.get(symbol) {
            Some(overrides) => SymbolLimits {
                max_order_qty: overrides
                    .max_order_qty
                    .unwrap_or(self.defaults.max_order_qty),
                max_notional_value: overrides
                    .max_notional_value
                    .unwrap_or(self.defaults.max_notional_value),
                confirm_above_notional: overrides
                    .confirm_above_notional
                    .unwrap_or(self.defaults.confirm_above_notional),
                max_trades_per_day: overrides
                    .max_trades_per_day
                    .unwrap_or(self.defaults.max_trades_per_day),
                max_trades_per_week: overrides
                    .max_trades_per_week
                    .unwrap_or(self.defaults.max_trades_per_week),
                max_trades_per_month: overrides
                    .max_trades_per_month
                    .unwrap_or(self.defaults.max_trades_per_month),
            },
            None => self.defaults.clone(),
        }
    }

    /// Returns a human-readable description of all limits for agent system prompts.
    pub fn describe_limits(&self) -> String {
        let mut out = String::from("Risk limits:\n");

        let _ = writeln!(out, "  Defaults:");
        let _ = writeln!(out, "    max_order_qty: {}", self.defaults.max_order_qty);
        let _ = writeln!(
            out,
            "    max_notional_value: {}",
            self.defaults.max_notional_value
        );
        let _ = writeln!(
            out,
            "    confirm_above_notional: {}",
            self.defaults.confirm_above_notional
        );
        let _ = writeln!(
            out,
            "    max_trades_per_day: {}",
            self.defaults.max_trades_per_day
        );
        let _ = writeln!(
            out,
            "    max_trades_per_week: {}",
            self.defaults.max_trades_per_week
        );
        let _ = writeln!(
            out,
            "    max_trades_per_month: {}",
            self.defaults.max_trades_per_month
        );

        for (symbol, overrides) in &self.symbols {
            let _ = writeln!(out, "  {symbol}:");
            if let Some(v) = overrides.max_order_qty {
                let _ = writeln!(out, "    max_order_qty: {v}");
            }
            if let Some(v) = overrides.max_notional_value {
                let _ = writeln!(out, "    max_notional_value: {v}");
            }
            if let Some(v) = overrides.confirm_above_notional {
                let _ = writeln!(out, "    confirm_above_notional: {v}");
            }
            if let Some(v) = overrides.max_trades_per_day {
                let _ = writeln!(out, "    max_trades_per_day: {v}");
            }
            if let Some(v) = overrides.max_trades_per_week {
                let _ = writeln!(out, "    max_trades_per_week: {v}");
            }
            if let Some(v) = overrides.max_trades_per_month {
                let _ = writeln!(out, "    max_trades_per_month: {v}");
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_json() -> &'static str {
        r#"{
            "defaults": {
                "max_order_qty": "1.0",
                "max_notional_value": "100000",
                "confirm_above_notional": "50000",
                "max_trades_per_day": 50,
                "max_trades_per_week": 200,
                "max_trades_per_month": 500
            },
            "symbols": {
                "BTC/USD": {
                    "max_order_qty": "0.5",
                    "max_notional_value": "50000",
                    "confirm_above_notional": "25000"
                },
                "ETH/USD": {
                    "max_order_qty": "10.0"
                }
            }
        }"#
    }

    #[test]
    fn parse_valid_config() {
        let config: RiskConfig = serde_json::from_str(sample_json()).unwrap();
        assert_eq!(config.defaults.max_order_qty, dec!(1.0));
        assert_eq!(config.defaults.max_notional_value, dec!(100000));
        assert_eq!(config.defaults.max_trades_per_day, 50);
        assert_eq!(config.symbols.len(), 2);
    }

    #[test]
    fn merge_symbol_overrides() {
        let config: RiskConfig = serde_json::from_str(sample_json()).unwrap();
        let btc = config.limits_for("BTC/USD");
        assert_eq!(btc.max_order_qty, dec!(0.5));
        assert_eq!(btc.max_notional_value, dec!(50000));
        assert_eq!(btc.confirm_above_notional, dec!(25000));
        // Rate limits inherit from defaults
        assert_eq!(btc.max_trades_per_day, 50);
        assert_eq!(btc.max_trades_per_week, 200);
        assert_eq!(btc.max_trades_per_month, 500);
    }

    #[test]
    fn partial_override_inherits_defaults() {
        let config: RiskConfig = serde_json::from_str(sample_json()).unwrap();
        let eth = config.limits_for("ETH/USD");
        assert_eq!(eth.max_order_qty, dec!(10.0));
        // Everything else from defaults
        assert_eq!(eth.max_notional_value, dec!(100000));
        assert_eq!(eth.confirm_above_notional, dec!(50000));
    }

    #[test]
    fn unknown_symbol_gets_defaults() {
        let config: RiskConfig = serde_json::from_str(sample_json()).unwrap();
        let sol = config.limits_for("SOL/USD");
        assert_eq!(sol.max_order_qty, dec!(1.0));
        assert_eq!(sol.max_notional_value, dec!(100000));
    }

    #[test]
    fn bad_json_returns_error() {
        let result = serde_json::from_str::<RiskConfig>("not json");
        assert!(result.is_err());
    }

    #[test]
    fn missing_symbols_section_ok() {
        let json = r#"{
            "defaults": {
                "max_order_qty": "1.0",
                "max_notional_value": "100000",
                "confirm_above_notional": "50000",
                "max_trades_per_day": 50,
                "max_trades_per_week": 200,
                "max_trades_per_month": 500
            }
        }"#;
        let config: RiskConfig = serde_json::from_str(json).unwrap();
        assert!(config.symbols.is_empty());
    }

    #[test]
    fn describe_limits_contains_defaults() {
        let config: RiskConfig = serde_json::from_str(sample_json()).unwrap();
        let desc = config.describe_limits();
        assert!(desc.contains("Defaults:"));
        assert!(desc.contains("max_order_qty: 1.0"));
        assert!(desc.contains("max_notional_value: 100000"));
    }

    #[test]
    fn describe_limits_contains_overrides() {
        let config: RiskConfig = serde_json::from_str(sample_json()).unwrap();
        let desc = config.describe_limits();
        assert!(desc.contains("BTC/USD:"));
        assert!(desc.contains("max_order_qty: 0.5"));
    }
}
