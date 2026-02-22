//! Simulation engine for paper-trading against live market data.
//!
//! When simulation mode is active, the engine intercepts orders that would
//! normally be sent to the Kraken exchange and fills them locally using the
//! current bid/ask from the ticker stream. Agents receive the same
//! [`AddOrderResponse`] and [`ExecutionUpdateResponse`] messages they would
//! from a real exchange, so they remain completely unaware of the simulation.

use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rust_decimal::Decimal;

use crate::models::add_order::{
    AddOrderParams, AddOrderResponse, AddOrderResult, OrderSide, OrderType,
};
use crate::models::execution::{ExecutionData, ExecutionUpdateResponse};
use crate::models::ticker::TickerData;

/// Kraken taker fee rate (0.26%).
const DEFAULT_FEE_RATE: Decimal = Decimal::from_parts(26, 0, 0, false, 4);

/// A completed simulated fill.
#[derive(Debug, Clone)]
pub struct SimulatedFill {
    pub order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub qty: Decimal,
    pub fill_price: Decimal,
    pub fee: Decimal,
    pub realized_pnl: Decimal,
    pub timestamp: String,
}

/// Engine that simulates order execution using live ticker data.
pub struct SimulationEngine {
    next_order_id: u64,
    next_exec_id: u64,
    sequence: i64,
    positions: HashMap<String, Decimal>,
    avg_entry_prices: HashMap<String, Decimal>,
    trade_history: Vec<SimulatedFill>,
    realized_pnl: Decimal,
    fee_rate: Decimal,
    session_start: Instant,
}

impl SimulationEngine {
    /// Creates a new simulation engine with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_order_id: 1,
            next_exec_id: 1,
            sequence: 1,
            positions: HashMap::new(),
            avg_entry_prices: HashMap::new(),
            trade_history: Vec::new(),
            realized_pnl: Decimal::ZERO,
            fee_rate: DEFAULT_FEE_RATE,
            session_start: Instant::now(),
        }
    }

    /// Executes an order against current ticker data.
    ///
    /// Returns synthesized exchange responses identical in shape to what
    /// the real Kraken WebSocket would produce.
    ///
    /// If no ticker data is available for the symbol, returns a failed
    /// `AddOrderResponse` with an error message.
    pub fn execute_order(
        &mut self,
        params: &AddOrderParams,
        ticker: Option<&TickerData>,
    ) -> (AddOrderResponse, Option<ExecutionUpdateResponse>) {
        let ticker = match ticker {
            Some(t) => t,
            None => {
                return (
                    self.make_failed_response(format!("no ticker data for {}", params.symbol)),
                    None,
                );
            }
        };

        let fill_price = match self.determine_fill_price(params, ticker) {
            Some(p) => p,
            None => {
                return (
                    self.make_failed_response(format!(
                        "cannot fill {:?} {:?} without limit price",
                        params.order_type, params.side
                    )),
                    None,
                );
            }
        };

        let order_id = self.next_order_id();
        let exec_id = self.next_exec_id();
        let timestamp = iso_timestamp();
        let qty = params.order_qty;
        let cost = qty * fill_price;
        let fee = cost * self.fee_rate;

        // Track position and compute realized P&L
        let realized = self.update_position(&params.symbol, &params.side, qty, fill_price);
        self.realized_pnl += realized - fee;

        let fill = SimulatedFill {
            order_id: order_id.clone(),
            symbol: params.symbol.clone(),
            side: params.side,
            qty,
            fill_price,
            fee,
            realized_pnl: realized - fee,
            timestamp: timestamp.clone(),
        };
        self.trade_history.push(fill);

        let order_response = AddOrderResponse {
            method: "add_order".to_string(),
            success: true,
            result: Some(AddOrderResult {
                order_id: order_id.clone(),
                cl_ord_id: params.cl_ord_id.clone(),
                order_userref: params.order_userref,
            }),
            error: None,
            time_in: timestamp.clone(),
            time_out: timestamp.clone(),
            req_id: None,
        };

        let side_str = match params.side {
            OrderSide::Buy => "buy",
            OrderSide::Sell => "sell",
        };
        let order_type_str = match params.order_type {
            OrderType::Market => "market",
            OrderType::Limit => "limit",
            _ => "market",
        };

        let seq = self.next_sequence();
        let execution = ExecutionUpdateResponse {
            channel: "executions".to_string(),
            tpe: "update".to_string(),
            sequence: seq,
            data: vec![ExecutionData {
                order_id: order_id.clone(),
                order_userref: params.order_userref,
                cl_ord_id: params.cl_ord_id.clone(),
                exec_id: Some(exec_id),
                trade_id: None,
                ord_ref_id: None,
                symbol: params.symbol.clone(),
                side: side_str.to_string(),
                order_type: order_type_str.to_string(),
                order_qty: qty,
                order_status: "filled".to_string(),
                time_in_force: None,
                limit_price: params.limit_price,
                limit_price_type: None,
                avg_price: Some(fill_price),
                last_price: Some(fill_price),
                cash_order_qty: None,
                exec_type: "filled".to_string(),
                last_qty: Some(qty),
                cum_qty: Some(qty),
                cum_cost: Some(cost),
                cost: Some(cost),
                liquidity_ind: Some("taker".to_string()),
                fees: Some(vec![crate::models::execution::Fee {
                    asset: "USD".to_string(),
                    qty: fee,
                }]),
                fee_ccy_pref: None,
                fee_usd_equiv: Some(fee),
                timestamp: timestamp.clone(),
                effective_time: Some(timestamp),
                expire_time: None,
                post_only: None,
                reduce_only: None,
                no_mpp: None,
                margin: None,
                margin_borrow: None,
                amended: None,
                liquidated: None,
                display_qty: None,
                display_qty_remain: None,
                triggers: None,
                contingent: None,
                reason: None,
                position_status: None,
                sender_sub_id: None,
            }],
        };

        (order_response, Some(execution))
    }

    /// Returns cumulative realized P&L (after fees).
    #[must_use]
    pub fn realized_pnl(&self) -> Decimal {
        self.realized_pnl
    }

    /// Computes unrealized P&L across all open positions using current tickers.
    #[must_use]
    pub fn unrealized_pnl(&self, tickers: &HashMap<String, TickerData>) -> Decimal {
        let mut pnl = Decimal::ZERO;
        for (symbol, &qty) in &self.positions {
            if qty == Decimal::ZERO {
                continue;
            }
            let entry = self
                .avg_entry_prices
                .get(symbol)
                .copied()
                .unwrap_or(Decimal::ZERO);
            if let Some(ticker) = tickers.get(symbol) {
                let mark = if qty > Decimal::ZERO {
                    ticker.bid
                } else {
                    ticker.ask
                };
                pnl += (mark - entry) * qty;
            }
        }
        pnl
    }

    /// Returns a snapshot of current positions (symbol -> net qty).
    #[must_use]
    pub fn positions(&self) -> &HashMap<String, Decimal> {
        &self.positions
    }

    /// Returns the average entry price for each position.
    #[must_use]
    pub fn avg_entry_prices(&self) -> &HashMap<String, Decimal> {
        &self.avg_entry_prices
    }

    /// Returns the history of simulated fills.
    #[must_use]
    pub fn trade_history(&self) -> &[SimulatedFill] {
        &self.trade_history
    }

    /// Returns the number of simulated trades executed.
    #[must_use]
    pub fn trade_count(&self) -> usize {
        self.trade_history.len()
    }

    /// Returns the elapsed session duration in seconds.
    #[must_use]
    pub fn session_secs(&self) -> u64 {
        self.session_start.elapsed().as_secs()
    }

    // -- Private helpers --

    fn next_order_id(&mut self) -> String {
        let id = format!("SIM-{:06}", self.next_order_id);
        self.next_order_id += 1;
        id
    }

    fn next_exec_id(&mut self) -> String {
        let id = format!("SIMX-{:06}", self.next_exec_id);
        self.next_exec_id += 1;
        id
    }

    fn next_sequence(&mut self) -> i64 {
        let seq = self.sequence;
        self.sequence += 1;
        seq
    }

    fn determine_fill_price(
        &self,
        params: &AddOrderParams,
        ticker: &TickerData,
    ) -> Option<Decimal> {
        match params.order_type {
            OrderType::Market => match params.side {
                OrderSide::Buy => Some(ticker.ask),
                OrderSide::Sell => Some(ticker.bid),
            },
            OrderType::Limit => {
                let limit = params.limit_price?;
                match params.side {
                    OrderSide::Buy => {
                        // Marketable if limit >= ask; fill at best available
                        if limit >= ticker.ask {
                            Some(ticker.ask)
                        } else {
                            Some(limit)
                        }
                    }
                    OrderSide::Sell => {
                        // Marketable if limit <= bid; fill at best available
                        if limit <= ticker.bid {
                            Some(ticker.bid)
                        } else {
                            Some(limit)
                        }
                    }
                }
            }
            // Unsupported order types fill at market price as a fallback
            _ => match params.side {
                OrderSide::Buy => Some(ticker.ask),
                OrderSide::Sell => Some(ticker.bid),
            },
        }
    }

    /// Updates position tracking and returns realized P&L (before fees).
    fn update_position(
        &mut self,
        symbol: &str,
        side: &OrderSide,
        qty: Decimal,
        fill_price: Decimal,
    ) -> Decimal {
        let current_pos = self.positions.get(symbol).copied().unwrap_or(Decimal::ZERO);
        let entry_price = self
            .avg_entry_prices
            .get(symbol)
            .copied()
            .unwrap_or(Decimal::ZERO);

        let signed_qty = match side {
            OrderSide::Buy => qty,
            OrderSide::Sell => -qty,
        };

        let new_pos = current_pos + signed_qty;
        let mut realized = Decimal::ZERO;

        // Check if this trade closes (or partially closes) an existing position
        let is_reducing = (current_pos > Decimal::ZERO && signed_qty < Decimal::ZERO)
            || (current_pos < Decimal::ZERO && signed_qty > Decimal::ZERO);

        if is_reducing {
            let close_qty = qty.min(current_pos.abs());
            if current_pos > Decimal::ZERO {
                // Closing long: profit = (fill - entry) * close_qty
                realized = (fill_price - entry_price) * close_qty;
            } else {
                // Closing short: profit = (entry - fill) * close_qty
                realized = (entry_price - fill_price) * close_qty;
            }

            // If position crossed zero, start new position at fill_price
            if (current_pos > Decimal::ZERO && new_pos < Decimal::ZERO)
                || (current_pos < Decimal::ZERO && new_pos > Decimal::ZERO)
            {
                self.avg_entry_prices.insert(symbol.to_string(), fill_price);
            } else if new_pos == Decimal::ZERO {
                self.avg_entry_prices.remove(symbol);
            }
            // If still same direction (partial close), entry price stays
        } else {
            // Adding to position: update weighted average entry price
            let total_cost = entry_price * current_pos.abs() + fill_price * qty;
            let total_qty = current_pos.abs() + qty;
            if total_qty != Decimal::ZERO {
                self.avg_entry_prices
                    .insert(symbol.to_string(), total_cost / total_qty);
            }
        }

        if new_pos == Decimal::ZERO {
            self.positions.remove(symbol);
        } else {
            self.positions.insert(symbol.to_string(), new_pos);
        }

        realized
    }

    fn make_failed_response(&self, error: String) -> AddOrderResponse {
        let ts = iso_timestamp();
        AddOrderResponse {
            method: "add_order".to_string(),
            success: false,
            result: None,
            error: Some(error),
            time_in: ts.clone(),
            time_out: ts,
            req_id: None,
        }
    }
}

impl Default for SimulationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Produces an ISO 8601 timestamp string from the current system time.
fn iso_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let micros = now.subsec_micros();

    // Convert epoch seconds to date/time components
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Civil date from days since epoch (algorithm from Howard Hinnant)
    let z = days as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z",
        y, m, d, hours, minutes, seconds, micros
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::add_order::{AddOrderBuilder, OrderSide};
    use rust_decimal_macros::dec;

    fn make_ticker(symbol: &str, bid: Decimal, ask: Decimal) -> TickerData {
        TickerData {
            symbol: symbol.to_string(),
            bid,
            bid_qty: dec!(10),
            ask,
            ask_qty: dec!(10),
            last: (bid + ask) / dec!(2),
            volume: dec!(1000),
            vwap: (bid + ask) / dec!(2),
            low: bid - dec!(100),
            high: ask + dec!(100),
            change: dec!(0),
            change_pct: dec!(0),
        }
    }

    fn make_market_buy(symbol: &str, qty: Decimal) -> AddOrderParams {
        AddOrderBuilder::market(OrderSide::Buy, symbol, qty)
            .build("sim-token")
            .unwrap()
    }

    fn make_market_sell(symbol: &str, qty: Decimal) -> AddOrderParams {
        AddOrderBuilder::market(OrderSide::Sell, symbol, qty)
            .build("sim-token")
            .unwrap()
    }

    fn make_limit_buy(symbol: &str, qty: Decimal, price: Decimal) -> AddOrderParams {
        AddOrderBuilder::limit(OrderSide::Buy, symbol, qty, price)
            .build("sim-token")
            .unwrap()
    }

    fn make_limit_sell(symbol: &str, qty: Decimal, price: Decimal) -> AddOrderParams {
        AddOrderBuilder::limit(OrderSide::Sell, symbol, qty, price)
            .build("sim-token")
            .unwrap()
    }

    #[test]
    fn market_buy_fills_at_ask() {
        let mut engine = SimulationEngine::new();
        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50010));
        let params = make_market_buy("BTC/USD", dec!(1));

        let (resp, exec) = engine.execute_order(&params, Some(&ticker));
        assert!(resp.success);
        assert!(resp.result.is_some());

        let exec = exec.unwrap();
        assert_eq!(exec.data[0].avg_price, Some(dec!(50010)));
        assert_eq!(exec.data[0].order_status, "filled");
    }

    #[test]
    fn market_sell_fills_at_bid() {
        let mut engine = SimulationEngine::new();
        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50010));
        let params = make_market_sell("BTC/USD", dec!(1));

        let (resp, exec) = engine.execute_order(&params, Some(&ticker));
        assert!(resp.success);

        let exec = exec.unwrap();
        assert_eq!(exec.data[0].avg_price, Some(dec!(50000)));
    }

    #[test]
    fn limit_buy_marketable_fills_at_ask() {
        let mut engine = SimulationEngine::new();
        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50010));
        // Limit above ask — fills at ask
        let params = make_limit_buy("BTC/USD", dec!(1), dec!(50020));

        let (_, exec) = engine.execute_order(&params, Some(&ticker));
        let exec = exec.unwrap();
        assert_eq!(exec.data[0].avg_price, Some(dec!(50010)));
    }

    #[test]
    fn limit_buy_non_marketable_fills_at_limit() {
        let mut engine = SimulationEngine::new();
        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50010));
        // Limit below ask — fills at limit price
        let params = make_limit_buy("BTC/USD", dec!(1), dec!(49990));

        let (_, exec) = engine.execute_order(&params, Some(&ticker));
        let exec = exec.unwrap();
        assert_eq!(exec.data[0].avg_price, Some(dec!(49990)));
    }

    #[test]
    fn limit_sell_marketable_fills_at_bid() {
        let mut engine = SimulationEngine::new();
        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50010));
        // Limit below bid — fills at bid
        let params = make_limit_sell("BTC/USD", dec!(1), dec!(49990));

        let (_, exec) = engine.execute_order(&params, Some(&ticker));
        let exec = exec.unwrap();
        assert_eq!(exec.data[0].avg_price, Some(dec!(50000)));
    }

    #[test]
    fn limit_sell_non_marketable_fills_at_limit() {
        let mut engine = SimulationEngine::new();
        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50010));
        // Limit above bid — fills at limit price
        let params = make_limit_sell("BTC/USD", dec!(1), dec!(50020));

        let (_, exec) = engine.execute_order(&params, Some(&ticker));
        let exec = exec.unwrap();
        assert_eq!(exec.data[0].avg_price, Some(dec!(50020)));
    }

    #[test]
    fn fee_calculation() {
        let mut engine = SimulationEngine::new();
        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50000));
        let params = make_market_buy("BTC/USD", dec!(1));

        let (_, exec) = engine.execute_order(&params, Some(&ticker));
        let exec = exec.unwrap();

        // Fee = 1 * 50000 * 0.0026 = 130.00
        let fee = exec.data[0].fees.as_ref().unwrap()[0].qty;
        assert_eq!(fee, dec!(130.0000));
    }

    #[test]
    fn position_tracking_and_pnl() {
        let mut engine = SimulationEngine::new();

        // Buy 1 BTC at 50000
        let buy_ticker = make_ticker("BTC/USD", dec!(50000), dec!(50000));
        let buy = make_market_buy("BTC/USD", dec!(1));
        engine.execute_order(&buy, Some(&buy_ticker));

        assert_eq!(engine.positions().get("BTC/USD"), Some(&dec!(1)));
        assert_eq!(engine.avg_entry_prices().get("BTC/USD"), Some(&dec!(50000)));

        // Sell 1 BTC at 51000 — close position
        let sell_ticker = make_ticker("BTC/USD", dec!(51000), dec!(51000));
        let sell = make_market_sell("BTC/USD", dec!(1));
        engine.execute_order(&sell, Some(&sell_ticker));

        // Position should be flat
        assert!(!engine.positions().contains_key("BTC/USD"));

        // Realized P&L = (51000 - 50000) * 1 - fees
        // Buy fee = 50000 * 0.0026 = 130
        // Sell fee = 51000 * 0.0026 = 132.60
        // First trade: realized = 0 (opening), pnl contribution = 0 - 130 = -130
        // Second trade: realized = 1000, pnl contribution = 1000 - 132.60 = 867.40
        // Total = -130 + 867.40 = 737.40
        assert_eq!(engine.realized_pnl(), dec!(737.4000));
    }

    #[test]
    fn partial_close_keeps_position() {
        let mut engine = SimulationEngine::new();

        // Buy 2 BTC at 50000
        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50000));
        let buy = make_market_buy("BTC/USD", dec!(2));
        engine.execute_order(&buy, Some(&ticker));

        // Sell 1 BTC at 51000 — partial close
        let sell_ticker = make_ticker("BTC/USD", dec!(51000), dec!(51000));
        let sell = make_market_sell("BTC/USD", dec!(1));
        engine.execute_order(&sell, Some(&sell_ticker));

        assert_eq!(engine.positions().get("BTC/USD"), Some(&dec!(1)));
        // Entry price should remain at 50000 for the remaining position
        assert_eq!(engine.avg_entry_prices().get("BTC/USD"), Some(&dec!(50000)));
    }

    #[test]
    fn position_crosses_zero() {
        let mut engine = SimulationEngine::new();

        // Buy 1 BTC at 50000
        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50000));
        let buy = make_market_buy("BTC/USD", dec!(1));
        engine.execute_order(&buy, Some(&ticker));

        // Sell 2 BTC at 51000 — close long + open short
        let sell_ticker = make_ticker("BTC/USD", dec!(51000), dec!(51000));
        let sell = make_market_sell("BTC/USD", dec!(2));
        engine.execute_order(&sell, Some(&sell_ticker));

        // Should be short 1 BTC with new entry at 51000
        assert_eq!(engine.positions().get("BTC/USD"), Some(&dec!(-1)));
        assert_eq!(engine.avg_entry_prices().get("BTC/USD"), Some(&dec!(51000)));
    }

    #[test]
    fn unrealized_pnl_computation() {
        let mut engine = SimulationEngine::new();

        // Buy 1 BTC at 50000
        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50000));
        let buy = make_market_buy("BTC/USD", dec!(1));
        engine.execute_order(&buy, Some(&ticker));

        // Current market: bid=51000, ask=51010
        let mut tickers = HashMap::new();
        tickers.insert(
            "BTC/USD".to_string(),
            make_ticker("BTC/USD", dec!(51000), dec!(51010)),
        );

        // Long position marked at bid
        let unrealized = engine.unrealized_pnl(&tickers);
        assert_eq!(unrealized, dec!(1000));
    }

    #[test]
    fn missing_ticker_returns_error() {
        let mut engine = SimulationEngine::new();
        let params = make_market_buy("BTC/USD", dec!(1));

        let (resp, exec) = engine.execute_order(&params, None);
        assert!(!resp.success);
        assert!(resp.error.is_some());
        assert!(exec.is_none());
    }

    #[test]
    fn order_ids_are_monotonic() {
        let mut engine = SimulationEngine::new();
        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50010));

        let params1 = make_market_buy("BTC/USD", dec!(1));
        let (resp1, _) = engine.execute_order(&params1, Some(&ticker));

        let params2 = make_market_buy("BTC/USD", dec!(1));
        let (resp2, _) = engine.execute_order(&params2, Some(&ticker));

        assert_eq!(resp1.result.unwrap().order_id, "SIM-000001");
        assert_eq!(resp2.result.unwrap().order_id, "SIM-000002");
    }

    #[test]
    fn trade_count_increments() {
        let mut engine = SimulationEngine::new();
        assert_eq!(engine.trade_count(), 0);

        let ticker = make_ticker("BTC/USD", dec!(50000), dec!(50010));
        let params = make_market_buy("BTC/USD", dec!(1));
        engine.execute_order(&params, Some(&ticker));

        assert_eq!(engine.trade_count(), 1);
    }

    #[test]
    fn weighted_average_entry_price() {
        let mut engine = SimulationEngine::new();

        // Buy 1 BTC at 50000
        let ticker1 = make_ticker("BTC/USD", dec!(50000), dec!(50000));
        let buy1 = make_market_buy("BTC/USD", dec!(1));
        engine.execute_order(&buy1, Some(&ticker1));

        // Buy 1 more BTC at 52000
        let ticker2 = make_ticker("BTC/USD", dec!(52000), dec!(52000));
        let buy2 = make_market_buy("BTC/USD", dec!(1));
        engine.execute_order(&buy2, Some(&ticker2));

        // Average entry = (50000 + 52000) / 2 = 51000
        assert_eq!(engine.avg_entry_prices().get("BTC/USD"), Some(&dec!(51000)));
        assert_eq!(engine.positions().get("BTC/USD"), Some(&dec!(2)));
    }

    #[test]
    fn iso_timestamp_format() {
        let ts = iso_timestamp();
        // Should look like "2024-01-15T12:00:00.000000Z"
        assert_eq!(ts.len(), 27);
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
        assert_eq!(&ts[19..20], ".");
    }
}
