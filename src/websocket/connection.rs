//! WebSocket connection lifecycle management.
//!
//! [`ConnectionManager`] handles connecting, reading messages, automatic
//! reconnection with exponential backoff, token refresh before expiry,
//! and re-subscription to all active channels after each reconnect.

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tungstenite::Message as WsMessage;

use super::{
    WsWriter, connect, ping, subscribe, subscribe_book, subscribe_executions, subscribe_instrument,
};
use crate::auth::get_websocket_token;
use crate::models::Channel;
use crate::models::book::BookDepth;
use crate::tui::Message;

/// Token is valid for 15 minutes; refresh 1 minute before expiry.
const TOKEN_REFRESH_INTERVAL: Duration = Duration::from_secs(14 * 60);

/// Initial backoff duration between reconnection attempts.
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

/// Maximum backoff duration between reconnection attempts.
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Commands sent from the main loop to the connection manager.
pub enum ConnectionCommand {
    /// A trading pair was subscribed in the UI.
    PairSubscribed(String),
    /// A trading pair was unsubscribed in the UI.
    PairUnsubscribed(String),
}

/// Why the reader loop exited.
enum DisconnectReason {
    /// The connection was lost or errored.
    ConnectionError,
    /// The auth token is about to expire and needs refreshing.
    TokenExpired,
    /// The message channel to the main loop was closed (app shutting down).
    Shutdown,
}

/// Manages the WebSocket connection lifecycle including reconnection
/// with exponential backoff and token refresh before expiry.
pub struct ConnectionManager {
    url: String,
    tls_config: Arc<rustls::ClientConfig>,
    api_key: Option<String>,
    api_secret: Option<String>,
    tx: mpsc::UnboundedSender<Message>,
    writer: Arc<tokio::sync::Mutex<Option<WsWriter>>>,
    cmd_rx: mpsc::UnboundedReceiver<ConnectionCommand>,
    subscribed_pairs: Vec<String>,
}

impl ConnectionManager {
    /// Creates a new connection manager.
    #[must_use]
    pub fn new(
        url: String,
        tls_config: Arc<rustls::ClientConfig>,
        api_key: Option<String>,
        api_secret: Option<String>,
        tx: mpsc::UnboundedSender<Message>,
        writer: Arc<tokio::sync::Mutex<Option<WsWriter>>>,
        cmd_rx: mpsc::UnboundedReceiver<ConnectionCommand>,
    ) -> Self {
        Self {
            url,
            tls_config,
            api_key,
            api_secret,
            tx,
            writer,
            cmd_rx,
            subscribed_pairs: Vec::new(),
        }
    }

    /// Returns `true` if API credentials are configured.
    fn has_credentials(&self) -> bool {
        matches!(
            (&self.api_key, &self.api_secret),
            (Some(k), Some(s)) if !k.is_empty() && !s.is_empty()
        )
    }

    /// Fetches a fresh auth token, or `None` if no credentials.
    async fn fetch_token(&self) -> Option<String> {
        if !self.has_credentials() {
            return None;
        }

        let key = self.api_key.as_deref().unwrap();
        let secret = self.api_secret.as_deref().unwrap();
        let tls = (*self.tls_config).clone();

        match get_websocket_token(key, secret, tls).await {
            Ok(token) => {
                info!("Fetched authentication token");
                Some(token)
            }
            Err(e) => {
                error!("Failed to fetch auth token: {e}");
                None
            }
        }
    }

    /// Subscribes to all tracked channels on the given writer.
    async fn resubscribe_all(&self, write: &mut WsWriter, token: Option<&str>) {
        if let Err(e) = subscribe_instrument(write).await {
            warn!("Failed to subscribe to instruments: {e}");
        }

        if !self.subscribed_pairs.is_empty() {
            for symbol in &self.subscribed_pairs {
                let symbols = vec![symbol.clone()];
                let _ = subscribe(write, &Channel::Ticker, &symbols, None).await;
                let _ = subscribe_book(write, &symbols, BookDepth::D25, None).await;
                let _ = subscribe(write, &Channel::Candles, &symbols, None).await;
                let _ = subscribe(write, &Channel::Trades, &symbols, None).await;
            }
        }

        if let Some(token) = token {
            if let Err(e) = subscribe_executions(write, token, true, false).await {
                warn!("Failed to subscribe to executions: {e}");
            }
        }
    }

    /// Runs the connection manager loop indefinitely.
    ///
    /// Connects to the WebSocket, reads messages, and automatically
    /// reconnects with exponential backoff on disconnection. Refreshes
    /// the auth token before it expires.
    pub async fn run(mut self) {
        let mut backoff = INITIAL_BACKOFF;

        loop {
            // Notify UI we're reconnecting (skip on first connect)
            let _ = self.tx.send(Message::Reconnecting);

            // Fetch a token if we have credentials
            let token = self.fetch_token().await;

            // Attempt connection
            info!(url = %self.url, "Connecting to WebSocket");
            let (mut write, read) = match connect(&self.url, self.tls_config.clone()).await {
                Ok(pair) => pair,
                Err(e) => {
                    error!("Connection failed: {e}");
                    let _ = self.tx.send(Message::Disconnected);
                    info!(backoff_secs = backoff.as_secs(), "Backing off before retry");
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                    continue;
                }
            };

            // Connected — ping and subscribe
            if let Err(e) = ping(&mut write).await {
                warn!("Ping failed: {e}");
                let _ = self.tx.send(Message::Disconnected);
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
                continue;
            }

            self.resubscribe_all(&mut write, token.as_deref()).await;

            // Hand the writer to the main loop
            {
                let mut guard = self.writer.lock().await;
                *guard = Some(write);
            }
            let _ = self.tx.send(Message::Connected);
            info!("WebSocket connected and subscribed");

            // Reset backoff on successful connection
            backoff = INITIAL_BACKOFF;

            // Enter reader loop
            let token_fetched_at = Instant::now();
            let reason = self
                .read_loop(read, token.as_deref(), token_fetched_at)
                .await;

            // Clear the writer so the main loop doesn't use a stale one
            {
                let mut guard = self.writer.lock().await;
                *guard = None;
            }

            match reason {
                DisconnectReason::TokenExpired => {
                    info!("Token expiring, reconnecting with fresh token");
                    // No backoff for planned refresh
                }
                DisconnectReason::ConnectionError => {
                    let _ = self.tx.send(Message::Disconnected);
                    info!(
                        backoff_secs = backoff.as_secs(),
                        "Connection lost, backing off"
                    );
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                }
                DisconnectReason::Shutdown => {
                    info!("Connection manager shutting down");
                    return;
                }
            }
        }
    }

    /// Reads messages from the WebSocket until disconnection, token
    /// expiry, or shutdown.
    async fn read_loop(
        &mut self,
        mut read: super::WsReader,
        token: Option<&str>,
        token_fetched_at: Instant,
    ) -> DisconnectReason {
        // Build the token refresh deadline
        let refresh_deadline = if token.is_some() {
            Some(tokio::time::Instant::from_std(
                token_fetched_at + TOKEN_REFRESH_INTERVAL,
            ))
        } else {
            None
        };

        let token_sleep = async {
            match refresh_deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => std::future::pending::<()>().await,
            }
        };
        tokio::pin!(token_sleep);

        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(WsMessage::Text(text))) => {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                                if let Some(message) = parse_ws_message(value) {
                                    if self.tx.send(message).is_err() {
                                        return DisconnectReason::Shutdown;
                                    }
                                }
                            }
                        }
                        Some(Ok(_)) => {} // Binary/Ping/Pong/Close frames
                        Some(Err(e)) => {
                            warn!("WebSocket error: {e}");
                            return DisconnectReason::ConnectionError;
                        }
                        None => {
                            warn!("WebSocket stream ended");
                            return DisconnectReason::ConnectionError;
                        }
                    }
                }

                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(ConnectionCommand::PairSubscribed(symbol)) => {
                            if !self.subscribed_pairs.contains(&symbol) {
                                self.subscribed_pairs.push(symbol);
                            }
                        }
                        Some(ConnectionCommand::PairUnsubscribed(symbol)) => {
                            self.subscribed_pairs.retain(|s| s != &symbol);
                        }
                        None => {
                            // Command channel closed, app is shutting down
                            return DisconnectReason::Shutdown;
                        }
                    }
                }

                () = &mut token_sleep => {
                    return DisconnectReason::TokenExpired;
                }
            }
        }
    }
}

/// Parses a WebSocket JSON message into a TUI [`Message`].
fn parse_ws_message(value: serde_json::Value) -> Option<Message> {
    let method = value.get("method").and_then(|m| m.as_str());
    let channel = value.get("channel").and_then(|c| c.as_str());
    let msg_type = value.get("type").and_then(|t| t.as_str());

    // Handle RPC responses
    if let Some(method) = method {
        return match method {
            "pong" => None,
            "add_order" => serde_json::from_value(value).ok().map(Message::OrderPlaced),
            "cancel_order" => serde_json::from_value(value)
                .ok()
                .map(Message::OrderCancelled),
            "amend_order" => serde_json::from_value(value)
                .ok()
                .map(Message::OrderAmended),
            "cancel_all" => serde_json::from_value(value)
                .ok()
                .map(Message::AllOrdersCancelled),
            _ => None,
        };
    }

    // Handle channel messages
    if let Some(channel) = channel {
        // Skip snapshots for data channels (except executions)
        if channel != "executions" && msg_type != Some("update") {
            return None;
        }

        return match channel {
            "heartbeat" => Some(Message::Heartbeat),
            "status" => serde_json::from_value(value).ok().map(Message::Status),
            "ticker" => serde_json::from_value(value).ok().map(Message::Ticker),
            "book" => serde_json::from_value(value).ok().map(Message::Book),
            "trade" => serde_json::from_value(value).ok().map(Message::Trade),
            "ohlc" => serde_json::from_value(value).ok().map(Message::Candle),
            "executions" => serde_json::from_value(value).ok().map(Message::Execution),
            _ => None,
        };
    }

    None
}
