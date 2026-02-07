//! Async WebSocket client for connecting to and interacting with the
//! Kraken WebSocket V2 API.

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::Message;
use tungstenite::Result;

use crate::models::book::BookUpdateResponse;
use crate::models::candle::CandleUpdateResponse;
use crate::models::instrument::InstrumentUpdateResponse;
use crate::models::ticker::TickerUpdateResponse;
use crate::models::trade::TradeUpdateResponse;
use crate::models::{
    Channel, ChannelLimits, Params, PingRequest, PongResponse, StatusUpdateResponse,
    SubscribeRequest, UnsubscribeRequest,
};

/// Write half of a Kraken WebSocket connection.
pub type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

/// Read half of a Kraken WebSocket connection.
pub type WsReader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// Establishes a WebSocket connection to the given URL.
///
/// # Errors
///
/// Returns a [`tungstenite::Error`] if the connection or TLS handshake fails.
pub async fn connect(url: String) -> Result<(WsWriter, WsReader)> {
    let (ws_stream, _) = connect_async(&url).await?;
    println!("Handshake successfully completed.");

    Ok(ws_stream.split())
}

/// Sends a ping message over the WebSocket to test connection liveness.
///
/// # Errors
///
/// Returns a [`tungstenite::Error`] if sending the message fails.
pub async fn ping(write: &mut WsWriter) -> Result<()> {
    let request = PingRequest {
        method: "ping".to_string(),
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize ping message.");
    write.send(Message::Text(json.into())).await?;

    Ok(())
}

/// Subscribes to a symbol-based channel (e.g., ticker, book, trades).
///
/// # Errors
///
/// Returns a [`tungstenite::Error`] if sending the subscription message fails.
pub async fn subscribe(write: &mut WsWriter, channel: &Channel, symbol: &[String]) -> Result<()> {
    let request = SubscribeRequest {
        method: "subscribe".to_string(),
        params: Params {
            channel: channel.as_str().to_string(),
            symbol: symbol.to_vec(),
        },
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize subscribe message.");
    write.send(Message::Text(json.into())).await?;
    println!("Subscribed to {} channel.", channel.as_str());

    Ok(())
}

/// Subscribes to the instrument channel (no symbol parameter required).
///
/// # Errors
///
/// Returns a [`tungstenite::Error`] if sending the subscription message fails.
pub async fn subscribe_instrument(write: &mut WsWriter) -> Result<()> {
    let json = serde_json::to_string(&serde_json::json!({
        "method": "subscribe",
        "params": { "channel": Channel::Instruments.as_str() }
    }))
    .expect("Failed to serialize subscribe message.");
    write.send(Message::Text(json.into())).await?;
    println!("Subscribed to {} channel.", Channel::Instruments.as_str());

    Ok(())
}

/// Unsubscribes from the instrument channel.
///
/// # Errors
///
/// Returns a [`tungstenite::Error`] if sending the unsubscribe message fails.
pub async fn unsubscribe_instrument(write: &mut WsWriter) -> Result<()> {
    let json = serde_json::to_string(&serde_json::json!({
        "method": "unsubscribe",
        "params": { "channel": Channel::Instruments.as_str() }
    }))
    .expect("Failed to serialize unsubscribe message.");
    write.send(Message::Text(json.into())).await?;
    println!(
        "Unsubscribed from {} channel.",
        Channel::Instruments.as_str()
    );

    Ok(())
}

/// Unsubscribes from a symbol-based channel.
///
/// # Errors
///
/// Returns a [`tungstenite::Error`] if sending the unsubscribe message fails.
pub async fn unsubscribe(write: &mut WsWriter, channel: &Channel, symbol: &[String]) -> Result<()> {
    let request = UnsubscribeRequest {
        method: "unsubscribe".to_string(),
        params: Params {
            channel: channel.as_str().to_string(),
            symbol: symbol.to_vec(),
        },
    };

    let json = serde_json::to_string(&request).expect("Failed to serialize unsubscribe message.");
    write.send(Message::Text(json.into())).await?;
    println!("Unsubscribed from {} channel.", channel.as_str());

    Ok(())
}

/// Reads and dispatches incoming WebSocket messages until all channel
/// limits are reached.
///
/// For each channel, up to `limits.<channel>` update messages are processed
/// and printed. Once a channel's limit is reached, it is automatically
/// unsubscribed. The function returns when every channel has hit its limit.
///
/// # Errors
///
/// Returns a [`tungstenite::Error`] if reading from or writing to the
/// WebSocket fails.
///
/// # Panics
///
/// Panics if a received message cannot be deserialized into the expected
/// response type (via `.expect()`).
pub async fn process_messages(
    write: &mut WsWriter,
    read: &mut WsReader,
    symbol: &[String],
    limits: &ChannelLimits,
) -> Result<()> {
    // Per-channel counters tracking how many updates have been received.
    let mut ticker_count = 0;
    let mut book_count = 0;
    let mut candle_count = 0;
    let mut trade_count = 0;
    let mut instrument_count = 0;

    while let Some(msg) = read.next().await {
        let msg = msg?;

        if let Message::Text(text) = msg {
            let value: serde_json::Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let msg_method = value.get("method").and_then(|m| m.as_str());
            let msg_type = value.get("type").and_then(|t| t.as_str());
            let msg_channel = value.get("channel").and_then(|c| c.as_str());

            if msg_method == Some("pong") {
                let response: PongResponse =
                    serde_json::from_value(value).expect("Failed to deserialize pong response.");

                println!(
                    "[{}] time_in: {} time_out: {}",
                    response.method, response.time_in, response.time_out
                );
                continue;
            }

            if msg_channel == Some(Channel::Heartbeat.as_str()) {
                continue;
            }

            if msg_channel == Some(Channel::Status.as_str()) {
                let response: StatusUpdateResponse =
                    serde_json::from_value(value).expect("Failed to deserialize status update.");

                println!("[{}][{}]", response.channel, response.tpe);
                for status in &response.data {
                    println!(
                        "  system: {} api: {} version: {} connection_id: {}",
                        status.system, status.api_version, status.version, status.connection_id
                    );
                }
                continue;
            }

            // Only process "update" messages; skip snapshots and other types.
            if msg_type != Some("update") {
                continue;
            }

            if msg_channel == Some(Channel::Ticker.as_str()) && ticker_count < limits.ticker {
                let response: TickerUpdateResponse =
                    serde_json::from_value(value).expect("Failed to deserialize ticker update.");

                println!("[{}][{}]", response.channel, response.tpe);
                for tick in &response.data {
                    println!(
                        "  [{}] bid: {} ({}) ask: {} ({}) last: {} vol: {} vwap: {} low: {} high: {} change: {} ({:.2}%)",
                        tick.symbol,
                        tick.bid,
                        tick.bid_qty,
                        tick.ask,
                        tick.ask_qty,
                        tick.last,
                        tick.volume,
                        tick.vwap,
                        tick.low,
                        tick.high,
                        tick.change,
                        tick.change_pct
                    );
                }

                ticker_count += 1;
                println!("Ticker tick {}/{}", ticker_count, limits.ticker);

                // Auto-unsubscribe once the limit is reached.
                if ticker_count >= limits.ticker {
                    unsubscribe(write, &Channel::Ticker, symbol).await?;
                }
            } else if msg_channel == Some(Channel::Book.as_str()) && book_count < limits.book {
                let response: BookUpdateResponse =
                    serde_json::from_value(value).expect("Failed to deserialize book update.");

                println!("[{}][{}]", response.channel, response.tpe);
                for entry in &response.data {
                    println!(
                        "  [{}] checksum: {} timestamp: {}",
                        entry.symbol, entry.checksum, entry.timestamp
                    );
                    for bid in &entry.bids {
                        println!("    bid: {} qty: {}", bid.price, bid.qty);
                    }
                    for ask in &entry.asks {
                        println!("    ask: {} qty: {}", ask.price, ask.qty);
                    }
                }

                book_count += 1;
                println!("Book tick {}/{}", book_count, limits.book);

                if book_count >= limits.book {
                    unsubscribe(write, &Channel::Book, symbol).await?;
                }
            } else if msg_channel == Some(Channel::Candles.as_str()) && candle_count < limits.candle
            {
                let response: CandleUpdateResponse =
                    serde_json::from_value(value).expect("Failed to deserialize candle update.");

                println!(
                    "[{}][{}] {}",
                    response.channel, response.tpe, response.timestamp
                );
                for candle in &response.data {
                    println!(
                        "  [{}] O: {} H: {} L: {} C: {} vwap: {} vol: {} trades: {} interval: {} begin: {} end: {}",
                        candle.symbol,
                        candle.open,
                        candle.high,
                        candle.low,
                        candle.close,
                        candle.vwap,
                        candle.volume,
                        candle.trades,
                        candle.interval,
                        candle.interval_begin,
                        candle.timestamp
                    );
                }

                candle_count += 1;
                println!("Candle tick {}/{}", candle_count, limits.candle);

                if candle_count >= limits.candle {
                    unsubscribe(write, &Channel::Candles, symbol).await?;
                }
            } else if msg_channel == Some(Channel::Trades.as_str()) && trade_count < limits.trade {
                let response: TradeUpdateResponse =
                    serde_json::from_value(value).expect("Failed to deserialize trade update.");

                println!("[{}][{}]", response.channel, response.tpe);
                for trade in &response.data {
                    println!(
                        "  [{}] {} {} @ {} qty: {} type: {} id: {} ts: {}",
                        trade.symbol,
                        trade.side,
                        trade.ord_type,
                        trade.price,
                        trade.qty,
                        trade.ord_type,
                        trade.trade_id,
                        trade.timestamp
                    );
                }

                trade_count += 1;
                println!("Trade tick {}/{}", trade_count, limits.trade);

                if trade_count >= limits.trade {
                    unsubscribe(write, &Channel::Trades, symbol).await?;
                }
            } else if msg_channel == Some(Channel::Instruments.as_str())
                && instrument_count < limits.instrument
            {
                let response: InstrumentUpdateResponse = serde_json::from_value(value)
                    .expect("Failed to deserialize instrument update.");

                println!("[{}][{}]", response.channel, response.tpe);
                println!(
                    "  assets: {} pairs: {}",
                    response.data.assets.len(),
                    response.data.pairs.len()
                );
                for asset in &response.data.assets {
                    println!(
                        "  [asset] {} status: {} precision: {} display: {} borrowable: {} collateral: {} margin_rate: {}",
                        asset.id,
                        asset.status,
                        asset.precision,
                        asset.precision_display,
                        asset.borrowable,
                        asset.collateral_value,
                        asset.margin_rate
                    );
                }
                for pair in &response.data.pairs {
                    println!(
                        "  [pair] {} {}/{} status: {} qty_prec: {} qty_inc: {} price_prec: {} price_inc: {} cost_prec: {} cost_min: {} qty_min: {} marginable: {} has_index: {}",
                        pair.symbol,
                        pair.base,
                        pair.quote,
                        pair.status,
                        pair.qty_precision,
                        pair.qty_increment,
                        pair.price_precision,
                        pair.price_increment,
                        pair.cost_precision,
                        pair.cost_min,
                        pair.qty_min,
                        pair.marginable,
                        pair.has_index
                    );
                    if let Some(margin) = pair.margin_initial {
                        println!(
                            "    margin_initial: {} long_limit: {:?} short_limit: {:?}",
                            margin, pair.position_limit_long, pair.position_limit_short
                        );
                    }
                }

                instrument_count += 1;
                println!("Instrument tick {}/{}", instrument_count, limits.instrument);

                if instrument_count >= limits.instrument {
                    unsubscribe_instrument(write).await?;
                }
            }

            // Exit once all channel limits have been reached.
            if ticker_count >= limits.ticker
                && book_count >= limits.book
                && candle_count >= limits.candle
                && trade_count >= limits.trade
                && instrument_count >= limits.instrument
            {
                break;
            }
        }
    }

    Ok(())
}
