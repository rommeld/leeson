//! Channel subscription and unsubscription operations.

use futures_util::SinkExt;
use tracing::{debug, info};
use tungstenite::Message;

use super::WsWriter;
use crate::Result;
use crate::models::book::BookDepth;
use crate::models::{
    BalancesSubscribeRequest, BalancesUnsubscribeRequest, BookSubscribeRequest, Channel,
    ExecutionsSubscribeRequest, ExecutionsUnsubscribeRequest, SubscribeRequest, UnsubscribeRequest,
};

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
    debug!("Sending subscribe request: {}", json);
    write.send(Message::Text(json.into())).await?;
    info!(
        channel = channel.as_str(),
        ?symbols,
        "Subscribed to channel"
    );

    Ok(())
}

/// Subscribes to the book channel with a specific depth.
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the subscription message fails.
pub async fn subscribe_book(
    write: &mut WsWriter,
    symbols: &[String],
    depth: BookDepth,
    token: Option<&str>,
) -> Result<()> {
    let request = BookSubscribeRequest::new(symbols, depth, token.map(String::from));
    let json = serde_json::to_string(&request)?;
    debug!("Sending book subscribe request: {}", json);
    write.send(Message::Text(json.into())).await?;
    info!(
        channel = Channel::Book.as_str(),
        depth = depth.as_u16(),
        ?symbols,
        "Subscribed to book channel"
    );

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

/// Subscribes to the balances channel (authenticated, no symbols).
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the subscription message fails.
pub async fn subscribe_balances(write: &mut WsWriter, token: &str, snapshot: bool) -> Result<()> {
    let request = BalancesSubscribeRequest::new(token, snapshot);
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        channel = Channel::Balances.as_str(),
        "Subscribed to channel"
    );

    Ok(())
}

/// Unsubscribes from the balances channel.
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the unsubscribe message fails.
pub async fn unsubscribe_balances(write: &mut WsWriter, token: &str) -> Result<()> {
    let request = BalancesUnsubscribeRequest::new(token);
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        channel = Channel::Balances.as_str(),
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
