use std::sync::Arc;

use tokio::sync::mpsc;

use leeson::LeesonError;
use leeson::config::fetch_config;
use leeson::models::Channel;
use leeson::models::book::BookDepth;
use leeson::tls::build_tls_config;
use leeson::tui::{self, App, Message};
use leeson::websocket::{ConnectionCommand, ConnectionManager, subscribe, subscribe_book};

#[tokio::main]
async fn main() -> Result<(), LeesonError> {
    let app_config = fetch_config()?;
    let tls_config = Arc::new(build_tls_config()?);

    let url = app_config.kraken.websocket_url.clone();
    let has_credentials = matches!(
        (&app_config.kraken.api_key, &app_config.kraken.api_secret),
        (Some(k), Some(s)) if !k.is_empty() && !s.is_empty()
    );

    // Setup terminal
    let mut terminal = tui::setup_terminal()?;

    // Create application state
    let mut app = App::new();
    app.authenticated = has_credentials;

    // Create message channel (shared with connection manager)
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Create command channel for subscription tracking
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<ConnectionCommand>();

    // Shared writer: connection manager writes, main loop reads
    let writer: Arc<tokio::sync::Mutex<Option<leeson::websocket::WsWriter>>> =
        Arc::new(tokio::sync::Mutex::new(None));

    // Spawn the connection manager
    let manager = ConnectionManager::new(
        url,
        tls_config,
        app_config.kraken.api_key.clone(),
        app_config.kraken.api_secret.clone(),
        tx.clone(),
        writer.clone(),
        cmd_rx,
    );
    tokio::spawn(async move { manager.run().await });

    // Spawn event reader for keyboard input
    tui::event::spawn_event_reader(tx.clone());

    // Spawn tick timer for periodic updates
    tui::event::spawn_tick_timer(tx.clone(), 250);

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
                        let mut guard = writer.lock().await;
                        if let Some(ref mut w) = *guard {
                            let symbols = vec![symbol.clone()];
                            let _ = subscribe(w, &Channel::Ticker, &symbols, None).await;
                            let _ = subscribe_book(w, &symbols, BookDepth::D25, None).await;
                            let _ = subscribe(w, &Channel::Candles, &symbols, None).await;
                            let _ = subscribe(w, &Channel::Trades, &symbols, None).await;
                        }
                        let _ = cmd_tx.send(ConnectionCommand::PairSubscribed(symbol));
                    }
                    tui::event::Action::UnsubscribePair(symbol) => {
                        let _ = cmd_tx.send(ConnectionCommand::PairUnsubscribed(symbol));
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
