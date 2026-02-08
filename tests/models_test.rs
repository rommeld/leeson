//! Deserialization tests for all Kraken WebSocket V2 model types.

use rust_decimal_macros::dec;

use leeson::models::amend_order::{AmendOrderResponse, AmendOrderResult};
use leeson::models::book::{BookData, BookUpdateResponse, PriceLevel};
use leeson::models::candle::{CandleData, CandleUpdateResponse};
use leeson::models::execution::{ExecutionData, ExecutionUpdateResponse, Fee};
use leeson::models::instrument::{AssetInfo, InstrumentData, InstrumentUpdateResponse, PairInfo};
use leeson::models::orders::{OrderEntry, OrdersData, OrdersUpdateResponse};
use leeson::models::ticker::{TickerData, TickerUpdateResponse};
use leeson::models::trade::{TradeData, TradeUpdateResponse};
use leeson::models::{HeartbeatResponse, PongResponse, StatusData, StatusUpdateResponse};

const TICKER_JSON: &str = include_str!("fixtures/ticker.json");
const BOOK_JSON: &str = include_str!("fixtures/book.json");
const TRADE_JSON: &str = include_str!("fixtures/trade.json");
const CANDLE_JSON: &str = include_str!("fixtures/candle.json");
const INSTRUMENT_JSON: &str = include_str!("fixtures/instrument.json");
const ORDERS_JSON: &str = include_str!("fixtures/orders.json");
const EXECUTION_JSON: &str = include_str!("fixtures/execution.json");
const PONG_JSON: &str = include_str!("fixtures/pong.json");
const STATUS_JSON: &str = include_str!("fixtures/status.json");
const HEARTBEAT_JSON: &str = include_str!("fixtures/heartbeat.json");
const AMEND_ORDER_SUCCESS_JSON: &str = include_str!("fixtures/amend_order_success.json");
const AMEND_ORDER_ERROR_JSON: &str = include_str!("fixtures/amend_order_error.json");

#[test]
fn test_ticker_update_response_deserializes() {
    let response: TickerUpdateResponse =
        serde_json::from_str(TICKER_JSON).expect("Failed to deserialize ticker response");

    assert_eq!(response.channel, "ticker");
    assert_eq!(response.tpe, "update");
    assert_eq!(response.data.len(), 1);

    let ticker: &TickerData = &response.data[0];
    assert_eq!(ticker.symbol, "BTC/USD");
    assert_eq!(ticker.bid, dec!(42150.5));
    assert_eq!(ticker.bid_qty, dec!(1.25));
    assert_eq!(ticker.ask, dec!(42155.0));
    assert_eq!(ticker.ask_qty, dec!(0.75));
    assert_eq!(ticker.last, dec!(42152.0));
    assert_eq!(ticker.volume, dec!(1250.5));
    assert_eq!(ticker.vwap, dec!(42100.25));
    assert_eq!(ticker.low, dec!(41800.0));
    assert_eq!(ticker.high, dec!(42500.0));
    assert_eq!(ticker.change, dec!(352.0));
    assert_eq!(ticker.change_pct, dec!(0.84));
}

#[test]
fn test_book_update_response_deserializes() {
    let response: BookUpdateResponse =
        serde_json::from_str(BOOK_JSON).expect("Failed to deserialize book response");

    assert_eq!(response.channel, "book");
    assert_eq!(response.tpe, "update");
    assert_eq!(response.data.len(), 1);

    let book: &BookData = &response.data[0];
    assert_eq!(book.symbol, "BTC/USD");
    assert_eq!(book.checksum, 1234567890);
    assert_eq!(book.timestamp, "2024-01-15T10:30:00.123456Z");

    assert_eq!(book.bids.len(), 2);
    let bid: &PriceLevel = &book.bids[0];
    assert_eq!(bid.price, dec!(42150.0));
    assert_eq!(bid.qty, dec!(1.5));

    assert_eq!(book.asks.len(), 2);
    let ask: &PriceLevel = &book.asks[0];
    assert_eq!(ask.price, dec!(42155.0));
    assert_eq!(ask.qty, dec!(0.75));
}

#[test]
fn test_trade_update_response_deserializes() {
    let response: TradeUpdateResponse =
        serde_json::from_str(TRADE_JSON).expect("Failed to deserialize trade response");

    assert_eq!(response.channel, "trade");
    assert_eq!(response.tpe, "update");
    assert_eq!(response.data.len(), 1);

    let trade: &TradeData = &response.data[0];
    assert_eq!(trade.symbol, "BTC/USD");
    assert_eq!(trade.side, "buy");
    assert_eq!(trade.price, dec!(42152.0));
    assert_eq!(trade.qty, dec!(0.5));
    assert_eq!(trade.ord_type, "market");
    assert_eq!(trade.trade_id, 987654321);
    assert_eq!(trade.timestamp, "2024-01-15T10:30:00.123456Z");
}

#[test]
fn test_candle_update_response_deserializes() {
    let response: CandleUpdateResponse =
        serde_json::from_str(CANDLE_JSON).expect("Failed to deserialize candle response");

    assert_eq!(response.channel, "ohlc");
    assert_eq!(response.tpe, "update");
    assert_eq!(response.timestamp, "2024-01-15T10:30:00.000000Z");
    assert_eq!(response.data.len(), 1);

    let candle: &CandleData = &response.data[0];
    assert_eq!(candle.symbol, "BTC/USD");
    assert_eq!(candle.open, dec!(42100.0));
    assert_eq!(candle.high, dec!(42200.0));
    assert_eq!(candle.low, dec!(42050.0));
    assert_eq!(candle.close, dec!(42152.0));
    assert_eq!(candle.vwap, dec!(42125.5));
    assert_eq!(candle.trades, 150);
    assert_eq!(candle.volume, dec!(25.5));
    assert_eq!(candle.interval_begin, "2024-01-15T10:00:00.000000Z");
    assert_eq!(candle.interval, 1);
}

#[test]
fn test_instrument_update_response_deserializes() {
    let response: InstrumentUpdateResponse =
        serde_json::from_str(INSTRUMENT_JSON).expect("Failed to deserialize instrument response");

    assert_eq!(response.channel, "instrument");
    assert_eq!(response.tpe, "update");

    let data: &InstrumentData = &response.data;
    assert_eq!(data.assets.len(), 2);
    assert_eq!(data.pairs.len(), 2);

    // Test asset with all fields
    let btc: &AssetInfo = &data.assets[0];
    assert_eq!(btc.id, "BTC");
    assert_eq!(btc.status, "enabled");
    assert_eq!(btc.precision, 10);
    assert_eq!(btc.precision_display, 5);
    assert!(btc.borrowable);
    assert_eq!(btc.collateral_value, dec!(0.95));
    assert_eq!(btc.margin_rate, dec!(0.02));

    // Test pair with optional margin fields
    let btc_usd: &PairInfo = &data.pairs[0];
    assert_eq!(btc_usd.symbol, "BTC/USD");
    assert_eq!(btc_usd.base, "BTC");
    assert_eq!(btc_usd.quote, "USD");
    assert_eq!(btc_usd.status, "online");
    assert!(btc_usd.marginable);
    assert_eq!(btc_usd.margin_initial, Some(dec!(0.2)));
    assert_eq!(btc_usd.position_limit_long, Some(100));
    assert_eq!(btc_usd.position_limit_short, Some(100));
    assert!(btc_usd.has_index);

    // Test pair without optional margin fields
    let eth_usd: &PairInfo = &data.pairs[1];
    assert_eq!(eth_usd.symbol, "ETH/USD");
    assert!(!eth_usd.marginable);
    assert_eq!(eth_usd.margin_initial, None);
    assert_eq!(eth_usd.position_limit_long, None);
    assert_eq!(eth_usd.position_limit_short, None);
    assert!(!eth_usd.has_index);
}

#[test]
fn test_orders_update_response_deserializes() {
    let response: OrdersUpdateResponse =
        serde_json::from_str(ORDERS_JSON).expect("Failed to deserialize orders response");

    assert_eq!(response.channel, "level3");
    assert_eq!(response.tpe, "update");
    assert_eq!(response.data.len(), 1);

    let orders: &OrdersData = &response.data[0];
    assert_eq!(orders.symbol, "BTC/USD");
    assert_eq!(orders.checksum, 9876543210);
    assert_eq!(orders.timestamp, "2024-01-15T10:30:00.123456Z");

    // Test bid with "add" event
    assert_eq!(orders.bids.len(), 2);
    let add_order: &OrderEntry = &orders.bids[0];
    assert_eq!(add_order.event, Some("add".to_string()));
    assert_eq!(add_order.order_id, "O123ABC");
    assert_eq!(add_order.limit_price, dec!(42150.0));
    assert_eq!(add_order.order_qty, dec!(1.5));

    // Test bid with "modify" event
    let modify_order: &OrderEntry = &orders.bids[1];
    assert_eq!(modify_order.event, Some("modify".to_string()));
    assert_eq!(modify_order.order_id, "O456DEF");

    // Test ask with "delete" event
    assert_eq!(orders.asks.len(), 1);
    let delete_order: &OrderEntry = &orders.asks[0];
    assert_eq!(delete_order.event, Some("delete".to_string()));
    assert_eq!(delete_order.order_id, "O789GHI");
    assert_eq!(delete_order.order_qty, dec!(0.0));
}

#[test]
fn test_execution_update_response_deserializes() {
    let response: ExecutionUpdateResponse =
        serde_json::from_str(EXECUTION_JSON).expect("Failed to deserialize execution response");

    assert_eq!(response.channel, "executions");
    assert_eq!(response.tpe, "update");
    assert_eq!(response.sequence, 42);
    assert_eq!(response.data.len(), 1);

    let exec: &ExecutionData = &response.data[0];
    assert_eq!(exec.order_id, "OABC12-DEFG3-HIJKL4");
    assert_eq!(exec.order_userref, Some(12345));
    assert_eq!(exec.cl_ord_id, Some("my-order-1".to_string()));
    assert_eq!(exec.exec_id, Some("TEXEC1-AAAAA-BBBBB".to_string()));
    assert_eq!(exec.trade_id, Some(987654));
    assert_eq!(exec.symbol, "BTC/USD");
    assert_eq!(exec.side, "buy");
    assert_eq!(exec.order_type, "limit");
    assert_eq!(exec.order_qty, dec!(1.5));
    assert_eq!(exec.order_status, "partially_filled");
    assert_eq!(exec.time_in_force, Some("GTC".to_string()));
    assert_eq!(exec.limit_price, Some(dec!(42150.0)));
    assert_eq!(exec.avg_price, Some(dec!(42148.5)));
    assert_eq!(exec.last_price, Some(dec!(42148.5)));
    assert_eq!(exec.exec_type, "trade");
    assert_eq!(exec.last_qty, Some(dec!(0.5)));
    assert_eq!(exec.cum_qty, Some(dec!(0.5)));
    assert_eq!(exec.cum_cost, Some(dec!(21074.25)));
    assert_eq!(exec.cost, Some(dec!(21074.25)));
    assert_eq!(exec.liquidity_ind, Some("m".to_string()));
    assert_eq!(exec.timestamp, "2024-01-15T10:30:00.123456Z");
    assert_eq!(exec.post_only, Some(true));
    assert_eq!(exec.reduce_only, Some(false));

    // Fees
    let fees: &Vec<Fee> = exec.fees.as_ref().expect("Expected fees");
    assert_eq!(fees.len(), 1);
    assert_eq!(fees[0].asset, "USD");
    assert_eq!(fees[0].qty, dec!(3.16));
    assert_eq!(exec.fee_usd_equiv, Some(dec!(3.16)));
}

#[test]
fn test_pong_response_deserializes() {
    let response: PongResponse =
        serde_json::from_str(PONG_JSON).expect("Failed to deserialize pong response");

    assert_eq!(response.method, "pong");
    assert_eq!(response.time_in, "2024-01-15T10:30:00.000000Z");
    assert_eq!(response.time_out, "2024-01-15T10:30:00.001234Z");
}

#[test]
fn test_status_update_response_deserializes() {
    let response: StatusUpdateResponse =
        serde_json::from_str(STATUS_JSON).expect("Failed to deserialize status response");

    assert_eq!(response.channel, "status");
    assert_eq!(response.tpe, "update");
    assert_eq!(response.data.len(), 1);

    let status: &StatusData = &response.data[0];
    assert_eq!(status.api_version, "v2");
    assert_eq!(status.connection_id, 12345678901234567);
    assert_eq!(status.system, "online");
    assert_eq!(status.version, "2.0.0");
}

#[test]
fn test_heartbeat_response_deserializes() {
    let response: HeartbeatResponse =
        serde_json::from_str(HEARTBEAT_JSON).expect("Failed to deserialize heartbeat response");

    assert_eq!(response.channel, "heartbeat");
}

#[test]
fn test_amend_order_success_response_deserializes() {
    let response: AmendOrderResponse = serde_json::from_str(AMEND_ORDER_SUCCESS_JSON)
        .expect("Failed to deserialize amend_order success response");

    assert!(response.success);
    assert_eq!(response.method, "amend_order");
    assert_eq!(response.time_in, "2024-07-26T13:39:04.922699Z");
    assert_eq!(response.time_out, "2024-07-26T13:39:04.924912Z");

    let result: &AmendOrderResult = response.result.as_ref().expect("Expected result");
    assert_eq!(result.amend_id, "TTW6PD-RC36L-ZZSWNU");
    assert_eq!(
        result.cl_ord_id,
        Some("2c6be801-1f53-4f79-a0bb-4ea1c95dfae9".to_string())
    );
    assert!(result.order_id.is_none());
    assert!(response.error.is_none());
}

#[test]
fn test_amend_order_error_response_deserializes() {
    let response: AmendOrderResponse = serde_json::from_str(AMEND_ORDER_ERROR_JSON)
        .expect("Failed to deserialize amend_order error response");

    assert!(!response.success);
    assert_eq!(response.method, "amend_order");
    assert_eq!(response.error, Some("EOrder:Unknown order".to_string()));
    assert!(response.result.is_none());
}
