//! Risk validation layer for order submission.
//!
//! Enforces configurable per-symbol limits on order quantity, notional value,
//! and trade frequency. Acts as a safety net between order creation and
//! WebSocket submission.

pub mod config;

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

use rust_decimal::Decimal;

use crate::models::add_order::AddOrderParams;
use config::RiskConfig;

/// Result of a successful risk check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskVerdict {
    /// Order is approved for immediate submission.
    Approved,
    /// Order requires operator confirmation before submission.
    RequiresConfirmation { reason: String },
}

/// Reason an order was rejected by the risk guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskCheckError {
    NonPositiveQuantity {
        qty: Decimal,
    },
    QuantityExceeded {
        qty: Decimal,
        max: Decimal,
        symbol: String,
    },
    NotionalExceeded {
        notional: Decimal,
        max: Decimal,
        symbol: String,
    },
    RateLimitExceeded {
        symbol: String,
        period: String,
        count: u32,
        max: u32,
    },
}

impl fmt::Display for RiskCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositiveQuantity { qty } => {
                write!(f, "order quantity must be positive, got {qty}")
            }
            Self::QuantityExceeded { qty, max, symbol } => {
                write!(f, "{symbol}: quantity {qty} exceeds max {max}")
            }
            Self::NotionalExceeded {
                notional,
                max,
                symbol,
            } => {
                write!(f, "{symbol}: notional value {notional} exceeds max {max}")
            }
            Self::RateLimitExceeded {
                symbol,
                period,
                count,
                max,
            } => {
                write!(
                    f,
                    "{symbol}: {count} trades in {period} exceeds limit of {max}"
                )
            }
        }
    }
}

impl std::error::Error for RiskCheckError {}

/// Tracks order submission timestamps per symbol for rate limiting.
struct RateTracker {
    submissions: HashMap<String, Vec<Instant>>,
}

impl RateTracker {
    fn new() -> Self {
        Self {
            submissions: HashMap::new(),
        }
    }

    fn record(&mut self, symbol: &str) {
        self.submissions
            .entry(symbol.to_string())
            .or_default()
            .push(Instant::now());
    }

    fn count_within(&self, symbol: &str, duration: Duration) -> u32 {
        let now = Instant::now();
        self.submissions
            .get(symbol)
            .map(|times| {
                times
                    .iter()
                    .filter(|t| now.duration_since(**t) <= duration)
                    .count() as u32
            })
            .unwrap_or(0)
    }

    /// Removes entries older than 30 days.
    fn prune(&mut self) {
        let cutoff = Duration::from_secs(30 * 24 * 60 * 60);
        let now = Instant::now();
        for times in self.submissions.values_mut() {
            times.retain(|t| now.duration_since(*t) <= cutoff);
        }
        self.submissions.retain(|_, times| !times.is_empty());
    }
}

const SECS_PER_DAY: u64 = 24 * 60 * 60;
const SECS_PER_WEEK: u64 = 7 * SECS_PER_DAY;
const SECS_PER_MONTH: u64 = 30 * SECS_PER_DAY;

/// Validates orders against configurable risk limits before submission.
pub struct RiskGuard {
    config: RiskConfig,
    tracker: RateTracker,
}

impl RiskGuard {
    /// Creates a new risk guard with the given configuration.
    pub fn new(config: RiskConfig) -> Self {
        Self {
            config,
            tracker: RateTracker::new(),
        }
    }

    /// Returns a reference to the risk configuration.
    pub fn config(&self) -> &RiskConfig {
        &self.config
    }

    /// Validates an order against all risk limits.
    ///
    /// Does NOT record the submission — call [`record_submission`] after
    /// the order is successfully sent to the exchange.
    pub fn check_order(&self, params: &AddOrderParams) -> Result<RiskVerdict, RiskCheckError> {
        let symbol = &params.symbol;
        let qty = params.order_qty;
        let limits = self.config.limits_for(symbol);

        // 1. Reject non-positive quantity
        if qty <= Decimal::ZERO {
            return Err(RiskCheckError::NonPositiveQuantity { qty });
        }

        // 2. Reject quantity exceeding max
        if qty > limits.max_order_qty {
            return Err(RiskCheckError::QuantityExceeded {
                qty,
                max: limits.max_order_qty,
                symbol: symbol.clone(),
            });
        }

        // 3. Check notional value (only if limit_price is present)
        if let Some(price) = params.limit_price {
            let notional = qty * price;
            if notional > limits.max_notional_value {
                return Err(RiskCheckError::NotionalExceeded {
                    notional,
                    max: limits.max_notional_value,
                    symbol: symbol.clone(),
                });
            }
        }

        // 4. Check rate limits
        let daily_count = self
            .tracker
            .count_within(symbol, Duration::from_secs(SECS_PER_DAY));
        if daily_count >= limits.max_trades_per_day {
            return Err(RiskCheckError::RateLimitExceeded {
                symbol: symbol.clone(),
                period: "day".to_string(),
                count: daily_count,
                max: limits.max_trades_per_day,
            });
        }

        let weekly_count = self
            .tracker
            .count_within(symbol, Duration::from_secs(SECS_PER_WEEK));
        if weekly_count >= limits.max_trades_per_week {
            return Err(RiskCheckError::RateLimitExceeded {
                symbol: symbol.clone(),
                period: "week".to_string(),
                count: weekly_count,
                max: limits.max_trades_per_week,
            });
        }

        let monthly_count = self
            .tracker
            .count_within(symbol, Duration::from_secs(SECS_PER_MONTH));
        if monthly_count >= limits.max_trades_per_month {
            return Err(RiskCheckError::RateLimitExceeded {
                symbol: symbol.clone(),
                period: "month".to_string(),
                count: monthly_count,
                max: limits.max_trades_per_month,
            });
        }

        // 5. Check if confirmation is needed (only if limit_price is present)
        if let Some(price) = params.limit_price {
            let notional = qty * price;
            if notional > limits.confirm_above_notional {
                return Ok(RiskVerdict::RequiresConfirmation {
                    reason: format!(
                        "notional value {notional} exceeds confirmation threshold {}",
                        limits.confirm_above_notional
                    ),
                });
            }
        }

        // 6. Approved
        Ok(RiskVerdict::Approved)
    }

    /// Records a successful order submission for rate limiting.
    pub fn record_submission(&mut self, symbol: &str) {
        self.tracker.record(symbol);
    }

    /// Prunes rate tracker entries older than 30 days.
    pub fn prune_stale_entries(&mut self) {
        self.tracker.prune();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RedactedToken;
    use crate::models::add_order::{AddOrderParams, OrderSide, OrderType};
    use rust_decimal_macros::dec;

    fn test_config() -> RiskConfig {
        serde_json::from_str(
            r#"{
                "defaults": {
                    "max_order_qty": "1.0",
                    "max_notional_value": "100000",
                    "confirm_above_notional": "50000",
                    "max_trades_per_day": 3,
                    "max_trades_per_week": 10,
                    "max_trades_per_month": 30
                },
                "symbols": {
                    "BTC/USD": {
                        "max_order_qty": "0.5"
                    }
                }
            }"#,
        )
        .unwrap()
    }

    fn make_params(symbol: &str, qty: Decimal, limit_price: Option<Decimal>) -> AddOrderParams {
        AddOrderParams {
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            symbol: symbol.to_string(),
            order_qty: qty,
            limit_price,
            time_in_force: None,
            expire_time: None,
            post_only: None,
            reduce_only: None,
            margin: None,
            cl_ord_id: None,
            order_userref: None,
            validate: None,
            triggers: None,
            conditional: None,
            display_qty: None,
            stp_type: None,
            fee_preference: None,
            no_mpp: None,
            token: RedactedToken::new("test"),
        }
    }

    fn make_market_params(symbol: &str, qty: Decimal) -> AddOrderParams {
        AddOrderParams {
            order_type: OrderType::Market,
            limit_price: None,
            ..make_params(symbol, qty, None)
        }
    }

    #[test]
    fn reject_zero_qty() {
        let guard = RiskGuard::new(test_config());
        let params = make_params("BTC/USD", dec!(0), Some(dec!(50000)));
        let result = guard.check_order(&params);
        assert!(matches!(
            result,
            Err(RiskCheckError::NonPositiveQuantity { .. })
        ));
    }

    #[test]
    fn reject_negative_qty() {
        let guard = RiskGuard::new(test_config());
        let params = make_params("BTC/USD", dec!(-0.1), Some(dec!(50000)));
        let result = guard.check_order(&params);
        assert!(matches!(
            result,
            Err(RiskCheckError::NonPositiveQuantity { .. })
        ));
    }

    #[test]
    fn reject_over_max_qty() {
        let guard = RiskGuard::new(test_config());
        // BTC/USD has max_order_qty of 0.5
        let params = make_params("BTC/USD", dec!(0.6), Some(dec!(50000)));
        let result = guard.check_order(&params);
        assert!(matches!(
            result,
            Err(RiskCheckError::QuantityExceeded { .. })
        ));
    }

    #[test]
    fn reject_over_max_notional() {
        let guard = RiskGuard::new(test_config());
        // ETH/USD defaults: max_notional = 100000, qty=1.0 * price=200000 = 200000 > 100000
        let params = make_params("ETH/USD", dec!(1.0), Some(dec!(200000)));
        let result = guard.check_order(&params);
        assert!(matches!(
            result,
            Err(RiskCheckError::NotionalExceeded { .. })
        ));
    }

    #[test]
    fn confirm_above_threshold() {
        let guard = RiskGuard::new(test_config());
        // Default confirm_above_notional is 50000; qty=0.4 * price=130000 = 52000 > 50000
        let params = make_params("BTC/USD", dec!(0.4), Some(dec!(130000)));
        let result = guard.check_order(&params);
        assert!(matches!(
            result,
            Ok(RiskVerdict::RequiresConfirmation { .. })
        ));
    }

    #[test]
    fn approve_valid_order() {
        let guard = RiskGuard::new(test_config());
        // qty=0.1 * price=50000 = 5000 — well within all limits
        let params = make_params("BTC/USD", dec!(0.1), Some(dec!(50000)));
        let result = guard.check_order(&params);
        assert_eq!(result, Ok(RiskVerdict::Approved));
    }

    #[test]
    fn rate_limit_after_n_submissions() {
        let mut guard = RiskGuard::new(test_config());
        let params = make_params("BTC/USD", dec!(0.1), Some(dec!(50000)));

        // Submit 3 orders (daily limit)
        for _ in 0..3 {
            assert_eq!(guard.check_order(&params), Ok(RiskVerdict::Approved));
            guard.record_submission("BTC/USD");
        }

        // 4th should be rejected
        let result = guard.check_order(&params);
        assert!(matches!(
            result,
            Err(RiskCheckError::RateLimitExceeded { .. })
        ));
    }

    #[test]
    fn market_orders_skip_notional_checks() {
        let guard = RiskGuard::new(test_config());
        // Market order with qty within limit but would exceed notional if price were checked
        let params = make_market_params("ETH/USD", dec!(0.5));
        let result = guard.check_order(&params);
        assert_eq!(result, Ok(RiskVerdict::Approved));
    }

    #[test]
    fn prune_does_not_panic_on_empty() {
        let mut guard = RiskGuard::new(test_config());
        guard.prune_stale_entries();
    }

    #[test]
    fn rate_limit_different_symbols_independent() {
        let mut guard = RiskGuard::new(test_config());

        // Fill BTC/USD daily limit
        for _ in 0..3 {
            guard.record_submission("BTC/USD");
        }

        // ETH/USD should still be allowed
        let params = make_params("ETH/USD", dec!(0.5), Some(dec!(1000)));
        assert_eq!(guard.check_order(&params), Ok(RiskVerdict::Approved));
    }

    #[test]
    fn display_errors() {
        let err = RiskCheckError::NonPositiveQuantity { qty: dec!(0) };
        assert_eq!(err.to_string(), "order quantity must be positive, got 0");

        let err = RiskCheckError::QuantityExceeded {
            qty: dec!(2.0),
            max: dec!(1.0),
            symbol: "BTC/USD".to_string(),
        };
        assert_eq!(err.to_string(), "BTC/USD: quantity 2.0 exceeds max 1.0");
    }
}
