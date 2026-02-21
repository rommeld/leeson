//! WebSocket connection lifecycle management.
//!
//! [`ConnectionManager`] handles connecting, reading messages, automatic
//! reconnection with exponential backoff, token refresh before expiry,
//! and re-subscription to all active channels after each reconnect.
//!
//! Maintains two connections:
//! - Public: `wss://ws.kraken.com/v2` for market data (ticker, book, ohlc, trade)
//! - Private: `wss://ws-auth.kraken.com/v2` for authenticated channels (executions, balances)

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use tungstenite::Message as WsMessage;
use zeroize::Zeroizing;

use super::{
    WsReader, WsWriter, connect, ping, subscribe, subscribe_balances, subscribe_book,
    subscribe_executions, subscribe_instrument,
};
use crate::auth::get_websocket_token;
use crate::models::Channel;
use crate::models::book::BookDepth;
use crate::tui::Message;
use crate::tui::app::TokenState;

/// Refresh token after 12 minutes (3-minute buffer before 15-min expiry).
const TOKEN_REFRESH_INTERVAL: Duration = Duration::from_secs(12 * 60);

/// Warn agents after 9 minutes that the token is aging.
const TOKEN_WARNING_THRESHOLD: Duration = Duration::from_secs(9 * 60);

/// Initial backoff duration between reconnection attempts.
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

/// Maximum backoff duration between reconnection attempts.
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Public WebSocket endpoint for market data.
const PUBLIC_WS_URL: &str = "wss://ws.kraken.com/v2";

/// Private WebSocket endpoint for authenticated channels.
const PRIVATE_WS_URL: &str = "wss://ws-auth.kraken.com/v2";

/// Commands sent from the main loop to the connection manager.
pub enum ConnectionCommand {
    /// A trading pair was subscribed in the UI.
    PairSubscribed(String),
    /// A trading pair was unsubscribed in the UI.
    PairUnsubscribed(String),
    /// The token was used to submit an authenticated request (e.g., order placement).
    TokenUsed,
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
///
/// Maintains two connections:
/// - Public: for market data (ticker, book, ohlc, trade)
/// - Private: for authenticated channels (executions, balances)
pub struct ConnectionManager {
    tls_config: Arc<rustls::ClientConfig>,
    api_key: Option<Zeroizing<String>>,
    api_secret: Option<Zeroizing<String>>,
    tx: mpsc::Sender<Message>,
    writer: Arc<tokio::sync::Mutex<Option<WsWriter>>>,
    cmd_rx: mpsc::Receiver<ConnectionCommand>,
    subscribed_pairs: Vec<String>,
    /// When the current token was last used for an authenticated operation.
    token_last_used: Option<Instant>,
}

impl ConnectionManager {
    /// Creates a new connection manager.
    ///
    /// Credentials are wrapped with [`Zeroizing`] to ensure secure erasure on drop.
    #[must_use]
    pub fn new(
        _url: String, // Ignored - we use fixed endpoints
        tls_config: Arc<rustls::ClientConfig>,
        api_key: Option<String>,
        api_secret: Option<String>,
        tx: mpsc::Sender<Message>,
        writer: Arc<tokio::sync::Mutex<Option<WsWriter>>>,
        cmd_rx: mpsc::Receiver<ConnectionCommand>,
    ) -> Self {
        Self {
            tls_config,
            api_key: api_key.map(Zeroizing::new),
            api_secret: api_secret.map(Zeroizing::new),
            tx,
            writer,
            cmd_rx,
            subscribed_pairs: Vec::new(),
            token_last_used: None,
        }
    }

    /// Sends a message to the TUI, logging a warning when the channel is full.
    fn try_send(&self, message: Message) {
        match self.tx.try_send(message) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!("message channel full, dropping message");
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                debug!("message channel closed");
            }
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

        let key = self.api_key.as_ref().unwrap().as_str();
        let secret = self.api_secret.as_ref().unwrap().as_str();
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

    /// Subscribes to public channels (market data) on the given writer.
    async fn subscribe_public(&self, write: &mut WsWriter) {
        if let Err(e) = subscribe_instrument(write).await {
            warn!("Failed to subscribe to instruments: {e}");
        }

        for symbol in &self.subscribed_pairs {
            let symbols = vec![symbol.clone()];
            let _ = subscribe(write, &Channel::Ticker, &symbols, None).await;
            let _ = subscribe_book(write, &symbols, BookDepth::D25, None).await;
            let _ = subscribe(write, &Channel::Candles, &symbols, None).await;
            let _ = subscribe(write, &Channel::Trades, &symbols, None).await;
        }
    }

    /// Subscribes to private channels (authenticated) on the given writer.
    async fn subscribe_private(&self, write: &mut WsWriter, token: &str) {
        if let Err(e) = subscribe_executions(write, token, true, false).await {
            warn!("Failed to subscribe to executions: {e}");
        }
        if let Err(e) = subscribe_balances(write, token, true).await {
            warn!("Failed to subscribe to balances: {e}");
        }
    }

    /// Runs the connection manager loop indefinitely.
    ///
    /// Connects to both public and private WebSocket endpoints,
    /// reads messages, and automatically reconnects with exponential
    /// backoff on disconnection. Refreshes the auth token before it expires.
    pub async fn run(mut self) {
        let mut backoff = INITIAL_BACKOFF;

        loop {
            // Notify UI we're reconnecting
            self.try_send(Message::Reconnecting);

            // Fetch a token if we have credentials (for private connection)
            let token = self.fetch_token().await;
            if token.is_some() {
                self.try_send(Message::TokenState(TokenState::Valid));
            } else {
                self.try_send(Message::TokenState(TokenState::Unavailable));
            }

            // Connect to PUBLIC endpoint for market data
            info!(url = %PUBLIC_WS_URL, "Connecting to public WebSocket");
            let public_result = connect(PUBLIC_WS_URL, self.tls_config.clone()).await;
            let (mut public_write, public_read) = match public_result {
                Ok(pair) => pair,
                Err(e) => {
                    error!("Public connection failed: {e}");
                    self.try_send(Message::Disconnected);
                    info!(backoff_secs = backoff.as_secs(), "Backing off before retry");
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                    continue;
                }
            };

            // Ping and subscribe on public connection
            if let Err(e) = ping(&mut public_write).await {
                warn!("Public ping failed: {e}");
                self.try_send(Message::Disconnected);
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
                continue;
            }

            self.subscribe_public(&mut public_write).await;
            info!("Public WebSocket connected and subscribed");

            // Connect to PRIVATE endpoint if we have credentials
            let private_connection = if let Some(ref token_str) = token {
                info!(url = %PRIVATE_WS_URL, "Connecting to private WebSocket");
                match connect(PRIVATE_WS_URL, self.tls_config.clone()).await {
                    Ok((mut private_write, private_read)) => {
                        if let Err(e) = ping(&mut private_write).await {
                            warn!("Private ping failed: {e}");
                            None
                        } else {
                            self.subscribe_private(&mut private_write, token_str).await;
                            info!("Private WebSocket connected and subscribed");
                            Some((private_write, private_read))
                        }
                    }
                    Err(e) => {
                        warn!("Private connection failed (continuing with public only): {e}");
                        None
                    }
                }
            } else {
                None
            };

            // Hand the public writer to the main loop (for sending subscriptions)
            {
                let mut guard = self.writer.lock().await;
                *guard = Some(public_write);
            }
            self.try_send(Message::Connected);

            // Reset backoff on successful connection
            backoff = INITIAL_BACKOFF;

            // Enter reader loop
            let token_fetched_at = Instant::now();
            let reason = self
                .read_loop(
                    public_read,
                    private_connection,
                    token.as_deref(),
                    token_fetched_at,
                )
                .await;

            // Clear the writer so the main loop doesn't use a stale one
            {
                let mut guard = self.writer.lock().await;
                *guard = None;
            }

            match reason {
                DisconnectReason::TokenExpired => {
                    self.try_send(Message::TokenState(TokenState::Refreshing));
                    info!("Token expiring, reconnecting with fresh token");
                    // No backoff for planned refresh
                }
                DisconnectReason::ConnectionError => {
                    self.try_send(Message::Disconnected);
                    self.try_send(Message::TokenState(TokenState::Refreshing));
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

    /// Reads messages from both WebSocket connections until disconnection,
    /// token expiry, or shutdown.
    async fn read_loop(
        &mut self,
        mut public_read: WsReader,
        private_connection: Option<(WsWriter, WsReader)>,
        token: Option<&str>,
        token_fetched_at: Instant,
    ) -> DisconnectReason {
        // Split private connection if available
        let mut private_read = private_connection.map(|(_, read)| read);

        // Build the token refresh deadline and warning deadline
        let refresh_deadline = if token.is_some() {
            Some(tokio::time::Instant::from_std(
                token_fetched_at + TOKEN_REFRESH_INTERVAL,
            ))
        } else {
            None
        };
        let warning_deadline = if token.is_some() {
            Some(tokio::time::Instant::from_std(
                token_fetched_at + TOKEN_WARNING_THRESHOLD,
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

        let warning_sleep = async {
            match warning_deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => std::future::pending::<()>().await,
            }
        };
        tokio::pin!(warning_sleep);

        loop {
            tokio::select! {
                // Read from public connection
                msg = public_read.next() => {
                    match msg {
                        Some(Ok(WsMessage::Text(text))) => {
                            debug!("Public WS message: {}", text);
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
                                && let Some(message) = parse_ws_message(value)
                            {
                                match self.tx.try_send(message) {
                                    Ok(()) => {}
                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                        warn!("message channel full, dropping public WS message");
                                    }
                                    Err(mpsc::error::TrySendError::Closed(_)) => {
                                        return DisconnectReason::Shutdown;
                                    }
                                }
                            }
                        }
                        Some(Ok(_)) => {} // Binary/Ping/Pong/Close frames
                        Some(Err(e)) => {
                            warn!("Public WebSocket error: {e}");
                            return DisconnectReason::ConnectionError;
                        }
                        None => {
                            warn!("Public WebSocket stream ended");
                            return DisconnectReason::ConnectionError;
                        }
                    }
                }

                // Read from private connection (if available)
                msg = async {
                    match &mut private_read {
                        Some(read) => read.next().await,
                        None => std::future::pending().await,
                    }
                } => {
                    match msg {
                        Some(Ok(WsMessage::Text(text))) => {
                            debug!("Private WS message: {}", text);
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
                                && let Some(message) = parse_ws_message(value)
                            {
                                match self.tx.try_send(message) {
                                    Ok(()) => {}
                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                        warn!("message channel full, dropping private WS message");
                                    }
                                    Err(mpsc::error::TrySendError::Closed(_)) => {
                                        return DisconnectReason::Shutdown;
                                    }
                                }
                            }
                        }
                        Some(Ok(_)) => {} // Binary/Ping/Pong/Close frames
                        Some(Err(e)) => {
                            warn!("Private WebSocket error: {e}");
                            // Don't fail completely, just log and continue with public
                            private_read = None;
                        }
                        None => {
                            warn!("Private WebSocket stream ended");
                            private_read = None;
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
                        Some(ConnectionCommand::TokenUsed) => {
                            self.token_last_used = Some(Instant::now());
                            debug!(
                                token_age_secs = token_fetched_at.elapsed().as_secs(),
                                "token used for authenticated operation"
                            );
                        }
                        None => {
                            // Command channel closed, app is shutting down
                            return DisconnectReason::Shutdown;
                        }
                    }
                }

                () = &mut warning_sleep => {
                    self.try_send(Message::TokenState(TokenState::ExpiringSoon));
                    info!(
                        token_age_secs = token_fetched_at.elapsed().as_secs(),
                        last_used = ?self.token_last_used.map(|t| t.elapsed()),
                        "token approaching expiry"
                    );
                    // warning_sleep is now completed — won't fire again
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
            "subscribe" => {
                // Log subscription responses but don't forward
                if let Some(success) = value.get("success").and_then(|s| s.as_bool())
                    && !success
                    && let Some(error) = value.get("error").and_then(|e| e.as_str())
                {
                    warn!("Subscription failed: {}", error);
                }
                None
            }
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
        // Channels that need both snapshots and updates
        // - ticker: snapshot for initial price, updates for changes
        // - book: snapshot for initial order book, updates for changes
        // - ohlc: snapshot for historical candles, updates for current candle
        // - executions/balances: authenticated channels need both
        // - trade: only updates (real-time trades as they happen)
        let needs_snapshot = matches!(
            channel,
            "ticker" | "book" | "ohlc" | "executions" | "balances"
        );

        // Skip snapshots for channels that only need updates (trade)
        if !needs_snapshot && msg_type != Some("update") {
            return None;
        }

        return match channel {
            "heartbeat" => Some(Message::Heartbeat),
            "status" => serde_json::from_value(value).ok().map(Message::Status),
            "ticker" => {
                debug!("Received ticker: {:?}", value);
                match serde_json::from_value(value.clone()) {
                    Ok(v) => Some(Message::Ticker(v)),
                    Err(e) => {
                        warn!("Failed to parse ticker: {e}");
                        debug!("Raw ticker: {}", value);
                        None
                    }
                }
            }
            "book" => {
                debug!("Received book: {:?}", value);
                match serde_json::from_value(value.clone()) {
                    Ok(v) => Some(Message::Book(v)),
                    Err(e) => {
                        warn!("Failed to parse book: {e}");
                        debug!("Raw book: {}", value);
                        None
                    }
                }
            }
            "trade" => {
                debug!("Received trade: {:?}", value);
                match serde_json::from_value(value.clone()) {
                    Ok(v) => Some(Message::Trade(v)),
                    Err(e) => {
                        warn!("Failed to parse trade: {e}");
                        debug!("Raw trade: {}", value);
                        None
                    }
                }
            }
            "ohlc" => {
                debug!("Received ohlc: {:?}", value);
                match serde_json::from_value(value.clone()) {
                    Ok(v) => Some(Message::Candle(v)),
                    Err(e) => {
                        warn!("Failed to parse ohlc: {e}");
                        debug!("Raw ohlc: {}", value);
                        None
                    }
                }
            }
            "executions" => serde_json::from_value(value).ok().map(Message::Execution),
            "balances" => {
                debug!("Received balances message: {:?}", value);
                match serde_json::from_value::<crate::models::balance::BalanceResponse>(
                    value.clone(),
                ) {
                    Ok(response) => Some(Message::Balance(response)),
                    Err(e) => {
                        warn!("Failed to parse balances message: {e}");
                        debug!("Raw balances payload: {}", value);
                        None
                    }
                }
            }
            _ => None,
        };
    }

    None
}
