use std::sync::Arc;

use tokio::sync::mpsc;

use leeson::LeesonError;
use leeson::auth::get_websocket_token;
use leeson::config::fetch_config;
use leeson::models::Channel;
use leeson::tls::build_tls_config;
use leeson::tui::{self, App, Message};
use leeson::websocket::{connect, ping, subscribe, subscribe_executions, subscribe_instrument};

#[tokio::main]
async fn main() -> Result<(), LeesonError> {
    let app_config = fetch_config()?;
    let tls_config = build_tls_config()?;

    let url = &app_config.kraken.websocket_url;

    // Fetch a WebSocket token if API credentials are configured.
    let token = match (&app_config.kraken.api_key, &app_config.kraken.api_secret) {
        (Some(key), Some(secret)) if !key.is_empty() && !secret.is_empty() => {
            Some(get_websocket_token(key, secret, tls_config.clone()).await?)
        }
        _ => None,
    };

    // Connect to WebSocket
    let (mut write, mut read) = connect(url, Arc::new(tls_config)).await?;
    ping(&mut write).await?;

    // Subscribe to instruments (always)
    subscribe_instrument(&mut write).await?;

    // If authenticated, subscribe to executions
    if let Some(ref token) = token {
        subscribe_executions(&mut write, token, true, false).await?;
    }

    // Setup terminal
    let mut terminal = tui::setup_terminal()?;

    // Create application state
    let mut app = App::new();
    app.authenticated = token.is_some();
    app.connection_status = tui::app::ConnectionStatus::Connected;

    // Create message channel
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Spawn event reader for keyboard input
    tui::event::spawn_event_reader(tx.clone());

    // Spawn tick timer for periodic updates
    tui::event::spawn_tick_timer(tx.clone(), 250);

    // Spawn WebSocket message reader
    let tx_ws = tx.clone();
    tokio::spawn(async move {
        use futures_util::StreamExt;
        use tungstenite::Message as WsMessage;

        while let Some(msg) = read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
                        && let Some(message) = parse_ws_message(value)
                        && tx_ws.send(message).is_err()
                    {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx_ws.send(Message::Disconnected);
                    break;
                }
                _ => {}
            }
        }
    });

    // Main event loop
    loop {
        // Render UI
        terminal
            .draw(|frame| tui::render(frame, &app))
            .map_err(|e| LeesonError::Io(e.to_string()))?;

        // Check if we should quit
        if app.should_quit {
            break;
        }

        // Wait for next message
        if let Some(message) = rx.recv().await {
            // Handle actions that require WebSocket writes
            if let Some(action) = tui::event::update(&mut app, message) {
                match action {
                    tui::event::Action::SubscribePair(symbol) => {
                        let symbols = vec![symbol];
                        let _ = subscribe(&mut write, &Channel::Ticker, &symbols, None).await;
                        let _ = subscribe(&mut write, &Channel::Book, &symbols, None).await;
                        let _ = subscribe(&mut write, &Channel::Candles, &symbols, None).await;
                        let _ = subscribe(&mut write, &Channel::Trades, &symbols, None).await;
                    }
                    tui::event::Action::UnsubscribePair(_symbol) => {
                        // TODO: Implement unsubscribe
                    }
                    tui::event::Action::SendToAgent1(message) => {
                        // Show user's message in Agent 1 output
                        app.add_agent_output(0, format!("You: {}", message));
                        // TODO: Send message to Agent 1 process and stream response
                        app.add_agent_output(
                            0,
                            "Agent 1: [Awaiting agent integration]".to_string(),
                        );
                    }
                    tui::event::Action::PlaceOrder => {
                        // TODO: Implement order placement
                    }
                    tui::event::Action::CancelOrder(_order_id) => {
                        // TODO: Implement order cancellation
                    }
                }
            }
        }
    }

    // Restore terminal
    tui::restore_terminal(&mut terminal)?;

    Ok(())
}

/// Parses a WebSocket JSON message into a TUI Message.
fn parse_ws_message(value: serde_json::Value) -> Option<Message> {
    // Extract routing fields
    let method = value.get("method").and_then(|m| m.as_str());
    let channel = value.get("channel").and_then(|c| c.as_str());
    let msg_type = value.get("type").and_then(|t| t.as_str());

    // Handle RPC responses
    if let Some(method) = method {
        return match method {
            "pong" => None, // Ignore pongs
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
