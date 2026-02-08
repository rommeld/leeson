//! Async WebSocket client for connecting to and interacting with the
//! Kraken WebSocket V2 API.

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, info, warn};
use tungstenite::Message;

use crate::Result;
use crate::error::LeesonError;

use crate::models::book::BookUpdateResponse;
use crate::models::candle::CandleUpdateResponse;
use crate::models::execution::ExecutionUpdateResponse;
use crate::models::instrument::InstrumentUpdateResponse;
use crate::models::orders::OrdersUpdateResponse;
use crate::models::ticker::TickerUpdateResponse;
use crate::models::trade::TradeUpdateResponse;
use crate::models::{
    AddOrderRequest, AddOrderResponse, AmendOrderRequest, AmendOrderResponse, CancelAllRequest,
    CancelAllResponse, CancelOrderRequest, CancelOrderResponse, Channel,
    ExecutionsSubscribeRequest, ExecutionsUnsubscribeRequest, PingRequest, PongResponse,
    StatusUpdateResponse, SubscribeRequest, UnsubscribeRequest,
};

/// Write half of a Kraken WebSocket connection.
pub type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

/// Read half of a Kraken WebSocket connection.
pub type WsReader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// Establishes a WebSocket connection to the given URL.
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if the connection or TLS handshake fails.
pub async fn connect(url: &str) -> Result<(WsWriter, WsReader)> {
    let (ws_stream, _) = connect_async(url).await?;
    info!("WebSocket handshake completed");

    Ok(ws_stream.split())
}

/// Sends a ping message over the WebSocket to test connection liveness.
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the message fails.
pub async fn ping(write: &mut WsWriter) -> Result<()> {
    let request = PingRequest::new();
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    debug!("Sent ping");

    Ok(())
}

/// Subscribes to a symbol-based channel (e.g., ticker, book, trades).
///
/// Pass a `token` for authenticated channels like `level3` (orders).
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the subscription message fails.
pub async fn subscribe(
    write: &mut WsWriter,
    channel: &Channel,
    symbols: &[String],
    token: Option<&str>,
) -> Result<()> {
    let request = SubscribeRequest::new(channel, symbols, token.map(String::from));
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(channel = channel.as_str(), "Subscribed to channel");

    Ok(())
}

/// Subscribes to the instrument channel (no symbol parameter required).
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the subscription message fails.
pub async fn subscribe_instrument(write: &mut WsWriter) -> Result<()> {
    let json = serde_json::to_string(&serde_json::json!({
        "method": "subscribe",
        "params": { "channel": Channel::Instruments.as_str() }
    }))?;
    write.send(Message::Text(json.into())).await?;
    info!(
        channel = Channel::Instruments.as_str(),
        "Subscribed to channel"
    );

    Ok(())
}

/// Unsubscribes from the instrument channel.
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the unsubscribe message fails.
pub async fn unsubscribe_instrument(write: &mut WsWriter) -> Result<()> {
    let json = serde_json::to_string(&serde_json::json!({
        "method": "unsubscribe",
        "params": { "channel": Channel::Instruments.as_str() }
    }))?;
    write.send(Message::Text(json.into())).await?;
    info!(
        channel = Channel::Instruments.as_str(),
        "Unsubscribed from channel"
    );

    Ok(())
}

/// Subscribes to the executions channel (authenticated, no symbols).
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the subscription message fails.
pub async fn subscribe_executions(
    write: &mut WsWriter,
    token: &str,
    snap_orders: bool,
    snap_trades: bool,
) -> Result<()> {
    let request = ExecutionsSubscribeRequest::new(token, snap_orders, snap_trades);
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        channel = Channel::Executions.as_str(),
        "Subscribed to channel"
    );

    Ok(())
}

/// Unsubscribes from the executions channel.
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the unsubscribe message fails.
pub async fn unsubscribe_executions(write: &mut WsWriter, token: &str) -> Result<()> {
    let request = ExecutionsUnsubscribeRequest::new(token);
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        channel = Channel::Executions.as_str(),
        "Unsubscribed from channel"
    );

    Ok(())
}

/// Unsubscribes from a symbol-based channel.
///
/// Pass a `token` for authenticated channels like `level3` (orders).
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the unsubscribe message fails.
pub async fn unsubscribe(
    write: &mut WsWriter,
    channel: &Channel,
    symbols: &[String],
    token: Option<&str>,
) -> Result<()> {
    let request = UnsubscribeRequest::new(channel, symbols, token.map(String::from));
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(channel = channel.as_str(), "Unsubscribed from channel");

    Ok(())
}

/// Sends an add_order request to place an order.
///
/// This is an RPC-style request that receives a single response indicating
/// success or failure. The response is handled in [`process_messages`].
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the request fails.
pub async fn add_order(write: &mut WsWriter, request: AddOrderRequest) -> Result<()> {
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        method = "add_order",
        req_id = ?request.req_id(),
        "Sent add_order request"
    );

    Ok(())
}

/// Sends a cancel_order request to cancel one or more orders.
///
/// This is an RPC-style request that receives a response for each order
/// being cancelled. The responses are handled in [`process_messages`].
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the request fails.
pub async fn cancel_order(write: &mut WsWriter, request: CancelOrderRequest) -> Result<()> {
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        method = "cancel_order",
        req_id = ?request.req_id(),
        "Sent cancel_order request"
    );

    Ok(())
}

/// Sends a cancel_all request to cancel all open orders.
///
/// This cancels all orders including untriggered orders and orders resting
/// in the book. The response is handled in [`process_messages`].
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the request fails.
pub async fn cancel_all(write: &mut WsWriter, request: CancelAllRequest) -> Result<()> {
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        method = "cancel_all",
        req_id = ?request.req_id(),
        "Sent cancel_all request"
    );

    Ok(())
}

/// Sends an amend_order request to modify an existing order in-place.
///
/// This is an RPC-style request that receives a single response indicating
/// success or failure. The response is handled in [`process_messages`].
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the request fails.
pub async fn amend_order(write: &mut WsWriter, request: AmendOrderRequest) -> Result<()> {
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        method = "amend_order",
        req_id = ?request.req_id(),
        "Sent amend_order request"
    );

    Ok(())
}

/// Reads and dispatches incoming WebSocket messages indefinitely.
///
/// Messages are parsed and logged via `tracing`. The function runs until
/// the WebSocket connection closes or an error occurs.
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if reading from the
/// WebSocket fails, or if a message cannot be deserialized into the
/// expected response type.
pub async fn process_messages(read: &mut WsReader) -> Result<()> {
    while let Some(msg) = read.next().await {
        let msg = msg?;

        if let Message::Text(text) = msg {
            let value: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| LeesonError::MalformedMessage(e.to_string()))?;

            let msg_method = value.get("method").and_then(|m| m.as_str());
            let msg_type = value.get("type").and_then(|t| t.as_str());
            let msg_channel = value.get("channel").and_then(|c| c.as_str());

            if msg_method == Some("pong") {
                let response: PongResponse = serde_json::from_value(value)?;
                debug!(
                    method = response.method,
                    time_in = response.time_in,
                    time_out = response.time_out,
                    "Received pong"
                );
                continue;
            }

            if msg_method == Some("add_order") {
                let response: AddOrderResponse = serde_json::from_value(value)?;
                if response.success {
                    if let Some(ref result) = response.result {
                        info!(
                            method = response.method,
                            order_id = result.order_id,
                            cl_ord_id = ?result.cl_ord_id,
                            req_id = ?response.req_id,
                            "Order placed successfully"
                        );
                    }
                } else {
                    warn!(
                        method = response.method,
                        error = ?response.error,
                        req_id = ?response.req_id,
                        "Order placement failed"
                    );
                }
                continue;
            }

            if msg_method == Some("cancel_order") {
                let response: CancelOrderResponse = serde_json::from_value(value)?;
                if response.success {
                    if let Some(ref result) = response.result {
                        info!(
                            method = response.method,
                            order_id = result.order_id,
                            cl_ord_id = ?result.cl_ord_id,
                            req_id = ?response.req_id,
                            "Order cancelled successfully"
                        );
                    }
                } else {
                    warn!(
                        method = response.method,
                        error = ?response.error,
                        req_id = ?response.req_id,
                        "Order cancellation failed"
                    );
                }
                continue;
            }

            if msg_method == Some("cancel_all") {
                let response: CancelAllResponse = serde_json::from_value(value)?;
                if response.success {
                    if let Some(ref result) = response.result {
                        info!(
                            method = response.method,
                            count = result.count,
                            req_id = ?response.req_id,
                            "All orders cancelled successfully"
                        );
                    }
                } else {
                    warn!(
                        method = response.method,
                        error = ?response.error,
                        req_id = ?response.req_id,
                        "Cancel all orders failed"
                    );
                }
                continue;
            }

            if msg_method == Some("amend_order") {
                let response: AmendOrderResponse = serde_json::from_value(value)?;
                if response.success {
                    if let Some(ref result) = response.result {
                        info!(
                            method = response.method,
                            amend_id = result.amend_id,
                            order_id = ?result.order_id,
                            cl_ord_id = ?result.cl_ord_id,
                            req_id = ?response.req_id,
                            "Order amended successfully"
                        );
                    }
                } else {
                    warn!(
                        method = response.method,
                        error = ?response.error,
                        req_id = ?response.req_id,
                        "Order amendment failed"
                    );
                }
                continue;
            }

            if msg_channel == Some(Channel::Heartbeat.as_str()) {
                debug!("Received heartbeat");
                continue;
            }

            if msg_channel == Some(Channel::Status.as_str()) {
                let response: StatusUpdateResponse = serde_json::from_value(value)?;
                for status in &response.data {
                    info!(
                        channel = response.channel,
                        msg_type = response.tpe,
                        system = status.system,
                        api_version = status.api_version,
                        version = status.version,
                        connection_id = status.connection_id,
                        "Status update"
                    );
                }
                continue;
            }

            // Process executions channel for both snapshots and updates.
            if msg_channel == Some(Channel::Executions.as_str()) {
                let response: ExecutionUpdateResponse = serde_json::from_value(value)?;
                for exec in &response.data {
                    info!(
                        channel = response.channel,
                        msg_type = response.tpe,
                        sequence = response.sequence,
                        exec_type = exec.exec_type,
                        order_id = exec.order_id,
                        symbol = exec.symbol,
                        side = exec.side,
                        order_type = exec.order_type,
                        order_status = exec.order_status,
                        order_qty = %exec.order_qty,
                        last_qty = exec.last_qty.map(|v| v.to_string()),
                        last_price = exec.last_price.map(|v| v.to_string()),
                        avg_price = exec.avg_price.map(|v| v.to_string()),
                        cum_qty = exec.cum_qty.map(|v| v.to_string()),
                        limit_price = exec.limit_price.map(|v| v.to_string()),
                        timestamp = exec.timestamp,
                        "Execution"
                    );
                }
                continue;
            }

            // Only process "update" messages; skip snapshots and other types.
            if msg_type != Some("update") {
                debug!(msg_type = msg_type, "Skipping non-update message");
                continue;
            }

            match msg_channel {
                Some(ch) if ch == Channel::Ticker.as_str() => {
                    let response: TickerUpdateResponse = serde_json::from_value(value)?;
                    for tick in &response.data {
                        info!(
                            channel = response.channel,
                            symbol = tick.symbol,
                            bid = %tick.bid,
                            bid_qty = %tick.bid_qty,
                            ask = %tick.ask,
                            ask_qty = %tick.ask_qty,
                            last = %tick.last,
                            volume = %tick.volume,
                            vwap = %tick.vwap,
                            low = %tick.low,
                            high = %tick.high,
                            change = %tick.change,
                            change_pct = %tick.change_pct,
                            "Ticker update"
                        );
                    }
                }
                Some(ch) if ch == Channel::Book.as_str() => {
                    let response: BookUpdateResponse = serde_json::from_value(value)?;
                    for entry in &response.data {
                        info!(
                            channel = response.channel,
                            symbol = entry.symbol,
                            checksum = entry.checksum,
                            timestamp = entry.timestamp,
                            bids = entry.bids.len(),
                            asks = entry.asks.len(),
                            "Book update"
                        );
                        for bid in &entry.bids {
                            debug!(price = %bid.price, qty = %bid.qty, "Bid");
                        }
                        for ask in &entry.asks {
                            debug!(price = %ask.price, qty = %ask.qty, "Ask");
                        }
                    }
                }
                Some(ch) if ch == Channel::Candles.as_str() => {
                    let response: CandleUpdateResponse = serde_json::from_value(value)?;
                    for candle in &response.data {
                        info!(
                            channel = response.channel,
                            symbol = candle.symbol,
                            open = %candle.open,
                            high = %candle.high,
                            low = %candle.low,
                            close = %candle.close,
                            vwap = %candle.vwap,
                            volume = %candle.volume,
                            trades = candle.trades,
                            interval = candle.interval,
                            interval_begin = candle.interval_begin,
                            timestamp = candle.timestamp,
                            "Candle update"
                        );
                    }
                }
                Some(ch) if ch == Channel::Trades.as_str() => {
                    let response: TradeUpdateResponse = serde_json::from_value(value)?;
                    for trade in &response.data {
                        info!(
                            channel = response.channel,
                            symbol = trade.symbol,
                            side = trade.side,
                            price = %trade.price,
                            qty = %trade.qty,
                            ord_type = trade.ord_type,
                            trade_id = trade.trade_id,
                            timestamp = trade.timestamp,
                            "Trade update"
                        );
                    }
                }
                Some(ch) if ch == Channel::Instruments.as_str() => {
                    let response: InstrumentUpdateResponse = serde_json::from_value(value)?;
                    info!(
                        channel = response.channel,
                        assets = response.data.assets.len(),
                        pairs = response.data.pairs.len(),
                        "Instrument update"
                    );
                    for asset in &response.data.assets {
                        debug!(
                            id = asset.id,
                            status = asset.status,
                            precision = asset.precision,
                            precision_display = asset.precision_display,
                            borrowable = asset.borrowable,
                            collateral_value = %asset.collateral_value,
                            margin_rate = %asset.margin_rate,
                            "Asset"
                        );
                    }
                    for pair in &response.data.pairs {
                        debug!(
                            symbol = pair.symbol,
                            base = pair.base,
                            quote = pair.quote,
                            status = pair.status,
                            qty_precision = pair.qty_precision,
                            qty_increment = %pair.qty_increment,
                            price_precision = pair.price_precision,
                            price_increment = %pair.price_increment,
                            cost_precision = pair.cost_precision,
                            cost_min = %pair.cost_min,
                            qty_min = %pair.qty_min,
                            marginable = pair.marginable,
                            has_index = pair.has_index,
                            "Pair"
                        );
                    }
                }
                Some(ch) if ch == Channel::Orders.as_str() => {
                    let response: OrdersUpdateResponse = serde_json::from_value(value)?;
                    for entry in &response.data {
                        info!(
                            channel = response.channel,
                            symbol = entry.symbol,
                            checksum = entry.checksum,
                            timestamp = entry.timestamp,
                            bids = entry.bids.len(),
                            asks = entry.asks.len(),
                            "Orders update"
                        );
                        for order in &entry.bids {
                            debug!(
                                event = order.event,
                                order_id = order.order_id,
                                limit_price = %order.limit_price,
                                order_qty = %order.order_qty,
                                timestamp = order.timestamp,
                                "Order bid"
                            );
                        }
                        for order in &entry.asks {
                            debug!(
                                event = order.event,
                                order_id = order.order_id,
                                limit_price = %order.limit_price,
                                order_qty = %order.order_qty,
                                timestamp = order.timestamp,
                                "Order ask"
                            );
                        }
                    }
                }
                Some(ch) => {
                    warn!(channel = ch, "Unknown channel");
                }
                None => {
                    warn!("Message missing channel field");
                }
            }
        }
    }

    Ok(())
}
