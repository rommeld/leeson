use rust_decimal_macros::dec;

use leeson::models::ticker::{TickerData, TickerUpdateResponse};

#[test]
fn deserialize_ticker_update_response() {
    let json = r#"{
        "channel": "ticker",
        "type": "update",
        "data": [
            {
                "symbol": "BTC/USD",
                "bid": 42150.50,
                "bid_qty": 1.25,
                "ask": 42155.00,
                "ask_qty": 0.75,
                "last": 42152.30,
                "volume": 1234.56789,
                "vwap": 42000.12,
                "low": 41500.00,
                "high": 42800.00,
                "change": 652.30,
                "change_pct": 1.57
            }
        ]
    }"#;

    let response: TickerUpdateResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.channel, "ticker");
    assert_eq!(response.tpe, "update");
    assert_eq!(response.data.len(), 1);

    let tick = &response.data[0];
    assert_eq!(tick.symbol, "BTC/USD");
    assert_eq!(tick.bid, dec!(42150.50));
    assert_eq!(tick.bid_qty, dec!(1.25));
    assert_eq!(tick.ask, dec!(42155.00));
    assert_eq!(tick.ask_qty, dec!(0.75));
    assert_eq!(tick.last, dec!(42152.30));
    assert_eq!(tick.volume, dec!(1234.56789));
    assert_eq!(tick.vwap, dec!(42000.12));
    assert_eq!(tick.low, dec!(41500.00));
    assert_eq!(tick.high, dec!(42800.00));
    assert_eq!(tick.change, dec!(652.30));
    assert_eq!(tick.change_pct, dec!(1.57));
}

#[test]
fn deserialize_ticker_data_directly() {
    let json = r#"{
        "symbol": "ETH/USD",
        "bid": 2250.10,
        "bid_qty": 10.5,
        "ask": 2251.00,
        "ask_qty": 8.3,
        "last": 2250.55,
        "volume": 45678.12,
        "vwap": 2240.00,
        "low": 2200.00,
        "high": 2300.00,
        "change": -15.45,
        "change_pct": -0.68
    }"#;

    let tick: TickerData = serde_json::from_str(json).unwrap();

    assert_eq!(tick.symbol, "ETH/USD");
    assert_eq!(tick.change, dec!(-15.45));
    assert_eq!(tick.change_pct, dec!(-0.68));
}

#[test]
fn deserialize_ticker_update_multiple_symbols() {
    let json = r#"{
        "channel": "ticker",
        "type": "update",
        "data": [
            {
                "symbol": "BTC/USD",
                "bid": 42000.0,
                "bid_qty": 1.0,
                "ask": 42001.0,
                "ask_qty": 1.0,
                "last": 42000.5,
                "volume": 100.0,
                "vwap": 42000.0,
                "low": 41000.0,
                "high": 43000.0,
                "change": 500.0,
                "change_pct": 1.2
            },
            {
                "symbol": "ETH/USD",
                "bid": 2200.0,
                "bid_qty": 5.0,
                "ask": 2201.0,
                "ask_qty": 5.0,
                "last": 2200.5,
                "volume": 500.0,
                "vwap": 2200.0,
                "low": 2100.0,
                "high": 2300.0,
                "change": -50.0,
                "change_pct": -2.2
            }
        ]
    }"#;

    let response: TickerUpdateResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.data.len(), 2);
    assert_eq!(response.data[0].symbol, "BTC/USD");
    assert_eq!(response.data[1].symbol, "ETH/USD");
}
