//! Order management RPC operations.
//!
//! Contains functions for placing, modifying, and cancelling orders via
//! the Kraken WebSocket V2 API.

use futures_util::SinkExt;
use tracing::info;
use tungstenite::Message;

use super::WsWriter;
use crate::Result;
use crate::models::{
    AddOrderRequest, AmendOrderRequest, BatchAddRequest, BatchCancelRequest, CancelAfterRequest,
    CancelAllRequest, CancelOrderRequest, EditOrderRequest,
};

/// Sends an add_order request to place an order.
///
/// This is an RPC-style request that receives a single response indicating
/// success or failure. The response is handled in [`process_messages`](super::handler::process_messages).
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

/// Sends a batch_add request to place multiple orders at once.
///
/// All orders in the batch must target the same currency pair. The batch
/// must contain between 2 and 15 orders. If validation fails for any order,
/// the entire batch is rejected.
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the request fails.
pub async fn batch_add(write: &mut WsWriter, request: BatchAddRequest) -> Result<()> {
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        method = "batch_add",
        order_count = request.order_count(),
        req_id = ?request.req_id(),
        "Sent batch_add request"
    );

    Ok(())
}

/// Sends a batch_cancel request to cancel multiple orders at once.
///
/// The batch must contain between 2 and 50 order identifiers. Orders can be
/// identified by `order_id` or `order_userref`.
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the request fails.
pub async fn batch_cancel(write: &mut WsWriter, request: BatchCancelRequest) -> Result<()> {
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        method = "batch_cancel",
        order_count = request.order_count(),
        req_id = ?request.req_id(),
        "Sent batch_cancel request"
    );

    Ok(())
}

/// Sends a cancel_order request to cancel one or more orders.
///
/// This is an RPC-style request that receives a response for each order
/// being cancelled.
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
/// in the book.
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

/// Sends a cancel_after request to set or refresh the dead man's switch.
///
/// This implements a "Dead Man's Switch" mechanism that automatically cancels
/// all orders if the countdown timer expires without being refreshed.
///
/// - Send with a timeout (e.g., 60 seconds) to start/refresh the countdown
/// - Send periodically (e.g., every 15-30 seconds) to keep orders active
/// - Send with timeout=0 to disable the feature
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the request fails.
pub async fn cancel_after(write: &mut WsWriter, request: CancelAfterRequest) -> Result<()> {
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        method = "cancel_all_orders_after",
        timeout = request.timeout(),
        req_id = ?request.req_id(),
        "Sent cancel_after request"
    );

    Ok(())
}

/// Sends an amend_order request to modify an existing order in-place.
///
/// This is an RPC-style request that receives a single response indicating
/// success or failure.
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

/// Sends an edit_order request to modify a live order.
///
/// When successful, the original order is cancelled and a new order is created
/// with the adjusted parameters and a new `order_id`. Note that this causes
/// loss of queue position. The `amend_order` endpoint is the preferred alternative
/// with fewer restrictions and better performance.
///
/// # Errors
///
/// Returns a [`LeesonError`](crate::LeesonError) if sending the request fails.
pub async fn edit_order(write: &mut WsWriter, request: EditOrderRequest) -> Result<()> {
    let json = serde_json::to_string(&request)?;
    write.send(Message::Text(json.into())).await?;
    info!(
        method = "edit_order",
        order_id = request.order_id(),
        req_id = ?request.req_id(),
        "Sent edit_order request"
    );

    Ok(())
}
