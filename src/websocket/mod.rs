//! Async WebSocket client for connecting to and interacting with the
//! Kraken WebSocket V2 API.
//!
//! This module is organized by domain:
//! - [`subscription`] - Channel subscribe/unsubscribe operations
//! - [`trading`] - Order management RPC operations
//! - [`handler`] - Incoming message processing

mod handler;
mod subscription;
mod trading;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, info};
use tungstenite::Message;

use crate::Result;
use crate::models::PingRequest;

// Re-export submodule functions at the crate level for convenience
pub use handler::process_messages;
pub use subscription::{
    subscribe, subscribe_executions, subscribe_instrument, unsubscribe, unsubscribe_executions,
    unsubscribe_instrument,
};
pub use trading::{
    add_order, amend_order, batch_add, batch_cancel, cancel_after, cancel_all, cancel_order,
    edit_order,
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
