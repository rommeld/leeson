//! Incoming WebSocket message processing.

use futures_util::StreamExt;
use tracing::{debug, info, warn};
use tungstenite::Message;

use super::WsReader;
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
    AddOrderResponse, AmendOrderResponse, BatchAddResponse, BatchCancelResponse,
    CancelAfterResponse, CancelAllResponse, CancelOrderResponse, Channel, EditOrderResponse,
    PongResponse, StatusUpdateResponse,
};

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

            dispatch_message(value)?;
        }
    }

    Ok(())
}

/// Routes a parsed JSON message to the appropriate handler.
fn dispatch_message(value: serde_json::Value) -> Result<()> {
    // Extract routing fields as owned strings to avoid borrow conflicts
    let msg_method = value
        .get("method")
        .and_then(|m| m.as_str())
        .map(String::from);
    let msg_type = value
        .get("type")
        .and_then(|t| t.as_str())
        .map(String::from);
    let msg_channel = value
        .get("channel")
        .and_then(|c| c.as_str())
        .map(String::from);

    // Handle RPC responses first (method-based routing)
    if let Some(ref method) = msg_method {
        return handle_rpc_response(method, value);
    }

    // Handle channel messages
    if let Some(ref channel) = msg_channel {
        return handle_channel_message(channel, msg_type.as_deref(), value);
    }

    Ok(())
}

/// Handles RPC-style responses (method-based).
fn handle_rpc_response(method: &str, value: serde_json::Value) -> Result<()> {
    match method {
        "pong" => {
            let response: PongResponse = serde_json::from_value(value)?;
            debug!(
                method = response.method,
                time_in = response.time_in,
                time_out = response.time_out,
                "Received pong"
            );
        }
        "add_order" => {
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
        }
        "batch_add" => {
            let response: BatchAddResponse = serde_json::from_value(value)?;
            if response.success {
                if let Some(ref results) = response.result {
                    info!(
                        method = response.method,
                        order_count = results.len(),
                        req_id = ?response.req_id,
                        "Batch orders placed successfully"
                    );
                    for result in results {
                        debug!(
                            order_id = result.order_id,
                            cl_ord_id = ?result.cl_ord_id,
                            order_userref = ?result.order_userref,
                            "Batch order"
                        );
                    }
                }
            } else {
                warn!(
                    method = response.method,
                    error = ?response.error,
                    req_id = ?response.req_id,
                    "Batch order placement failed"
                );
            }
        }
        "batch_cancel" => {
            let response: BatchCancelResponse = serde_json::from_value(value)?;
            if response.success {
                if let Some(ref result) = response.result {
                    info!(
                        method = response.method,
                        count = result.count,
                        req_id = ?response.req_id,
                        "Batch orders cancelled successfully"
                    );
                }
            } else {
                warn!(
                    method = response.method,
                    error = ?response.error,
                    req_id = ?response.req_id,
                    "Batch order cancellation failed"
                );
            }
        }
        "cancel_order" => {
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
        }
        "cancel_all" => {
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
        }
        "cancel_all_orders_after" => {
            let response: CancelAfterResponse = serde_json::from_value(value)?;
            if response.success {
                if let Some(ref result) = response.result {
                    info!(
                        method = response.method,
                        current_time = result.current_time,
                        trigger_time = result.trigger_time,
                        req_id = ?response.req_id,
                        "Dead man's switch set successfully"
                    );
                }
            } else {
                warn!(
                    method = response.method,
                    error = ?response.error,
                    req_id = ?response.req_id,
                    "Dead man's switch failed"
                );
            }
        }
        "amend_order" => {
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
        }
        "edit_order" => {
            let response: EditOrderResponse = serde_json::from_value(value)?;
            if response.success {
                if let Some(ref result) = response.result {
                    info!(
                        method = response.method,
                        order_id = result.order_id,
                        original_order_id = result.original_order_id,
                        req_id = ?response.req_id,
                        "Order edited successfully"
                    );
                }
            } else {
                warn!(
                    method = response.method,
                    error = ?response.error,
                    req_id = ?response.req_id,
                    "Order edit failed"
                );
            }
        }
        _ => {
            warn!(method = method, "Unknown RPC method");
        }
    }

    Ok(())
}

/// Handles channel-based messages (subscriptions).
fn handle_channel_message(
    channel: &str,
    msg_type: Option<&str>,
    value: serde_json::Value,
) -> Result<()> {
    // Check for unknown RPC-style messages that slipped through
    if value.get("method").is_some() {
        return Ok(());
    }
    // System channels (always process)
    if channel == Channel::Heartbeat.as_str() {
        debug!("Received heartbeat");
        return Ok(());
    }

    if channel == Channel::Status.as_str() {
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
        return Ok(());
    }

    // Executions channel (process both snapshots and updates)
    if channel == Channel::Executions.as_str() {
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
        return Ok(());
    }

    // Data channels (only process updates, skip snapshots)
    if msg_type != Some("update") {
        debug!(msg_type = msg_type, "Skipping non-update message");
        return Ok(());
    }

    handle_data_channel_update(channel, value)
}

/// Handles data channel update messages.
fn handle_data_channel_update(channel: &str, value: serde_json::Value) -> Result<()> {
    match channel {
        ch if ch == Channel::Ticker.as_str() => {
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
        ch if ch == Channel::Book.as_str() => {
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
        ch if ch == Channel::Candles.as_str() => {
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
        ch if ch == Channel::Trades.as_str() => {
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
        ch if ch == Channel::Instruments.as_str() => {
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
        ch if ch == Channel::Orders.as_str() => {
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
        _ => {
            warn!(channel = channel, "Unknown channel");
        }
    }

    Ok(())
}
