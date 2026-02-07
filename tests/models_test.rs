//! Deserialization tests for all Kraken WebSocket V2 model types.

use leeson::models::book::{BookData, BookUpdateResponse, PriceLevel};
use leeson::models::candle::{CandleData, CandleUpdateResponse};
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
const PONG_JSON: &str = include_str!("fixtures/pong.json");
const STATUS_JSON: &str = include_str!("fixtures/status.json");
const HEARTBEAT_JSON: &str = include_str!("fixtures/heartbeat.json");

#[test]
fn test_ticker_update_response_deserializes() {
    let response: TickerUpdateResponse =
        serde_json::from_str(TICKER_JSON).expect("Failed to deserialize ticker response");

    assert_eq!(response.channel, "ticker");
    assert_eq!(response.tpe, "update");
    assert_eq!(response.data.len(), 1);

    let ticker: &TickerData = &response.data[0];
    assert_eq!(ticker.symbol, "BTC/USD");
    assert_eq!(ticker.bid, 42150.5);
    assert_eq!(ticker.bid_qty, 1.25);
    assert_eq!(ticker.ask, 42155.0);
    assert_eq!(ticker.ask_qty, 0.75);
    assert_eq!(ticker.last, 42152.0);
    assert_eq!(ticker.volume, 1250.5);
    assert_eq!(ticker.vwap, 42100.25);
    assert_eq!(ticker.low, 41800.0);
    assert_eq!(ticker.high, 42500.0);
    assert_eq!(ticker.change, 352.0);
    assert_eq!(ticker.change_pct, 0.84);
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
    assert_eq!(bid.price, 42150.0);
    assert_eq!(bid.qty, 1.5);

    assert_eq!(book.asks.len(), 2);
    let ask: &PriceLevel = &book.asks[0];
    assert_eq!(ask.price, 42155.0);
    assert_eq!(ask.qty, 0.75);
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
    assert_eq!(trade.price, 42152.0);
    assert_eq!(trade.qty, 0.5);
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
    assert_eq!(candle.open, 42100.0);
    assert_eq!(candle.high, 42200.0);
    assert_eq!(candle.low, 42050.0);
    assert_eq!(candle.close, 42152.0);
    assert_eq!(candle.vwap, 42125.5);
    assert_eq!(candle.trades, 150);
    assert_eq!(candle.volume, 25.5);
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
    assert_eq!(btc.collateral_value, 0.95);
    assert_eq!(btc.margin_rate, 0.02);

    // Test pair with optional margin fields
    let btc_usd: &PairInfo = &data.pairs[0];
    assert_eq!(btc_usd.symbol, "BTC/USD");
    assert_eq!(btc_usd.base, "BTC");
    assert_eq!(btc_usd.quote, "USD");
    assert_eq!(btc_usd.status, "online");
    assert!(btc_usd.marginable);
    assert_eq!(btc_usd.margin_initial, Some(0.2));
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
    assert_eq!(add_order.limit_price, 42150.0);
    assert_eq!(add_order.order_qty, 1.5);

    // Test bid with "modify" event
    let modify_order: &OrderEntry = &orders.bids[1];
    assert_eq!(modify_order.event, Some("modify".to_string()));
    assert_eq!(modify_order.order_id, "O456DEF");

    // Test ask with "delete" event
    assert_eq!(orders.asks.len(), 1);
    let delete_order: &OrderEntry = &orders.asks[0];
    assert_eq!(delete_order.event, Some("delete".to_string()));
    assert_eq!(delete_order.order_id, "O789GHI");
    assert_eq!(delete_order.order_qty, 0.0);
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
