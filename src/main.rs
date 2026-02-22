use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use leeson::LeesonError;
use leeson::agent::{AgentCommand, AgentHandle, spawn_agent};
use leeson::auth::validate_credentials;
use leeson::config::fetch_config;
use leeson::models::Channel;
use leeson::models::add_order::AddOrderRequest;
use leeson::models::book::BookDepth;
use leeson::risk::RiskGuard;
use leeson::risk::config::{AgentRiskParams, RiskConfig};
use leeson::tls::build_tls_config;
use leeson::tui::app::{Mode, PendingOrder};
use leeson::tui::{self, App, Message};
use leeson::websocket::{
    ConnectionCommand, ConnectionManager, add_order, subscribe, subscribe_book, unsubscribe,
};

#[tokio::main]
async fn main() -> Result<(), LeesonError> {
    let app_config = fetch_config()?;
    let tls_config = Arc::new(build_tls_config()?);

    // Load risk configuration (required — running without risk limits is a hard error)
    let risk_config = RiskConfig::load(Path::new("risk.json"))?;
    let mut risk_guard = RiskGuard::new(risk_config);

    let has_credentials = matches!(
        (&app_config.kraken.api_key, &app_config.kraken.api_secret),
        (Some(k), Some(s)) if !k.is_empty() && !s.is_empty()
    );

    // Validate API credentials if provided
    let (credentials_valid, auth_error) = if has_credentials {
        let key = app_config.kraken.api_key.as_deref().unwrap();
        let secret = app_config.kraken.api_secret.as_deref().unwrap();
        match validate_credentials(key, secret, (*tls_config).clone()).await {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        }
    } else {
        (false, None)
    };

    // Use authenticated endpoint if credentials are valid, otherwise use configured URL
    // The balances channel requires the authenticated endpoint
    let url = if credentials_valid {
        "wss://ws-auth.kraken.com/v2".to_string()
    } else {
        app_config.kraken.websocket_url.clone()
    };

    // Load agent risk parameters (defaults if file missing)
    let agent_risk_path = Path::new("agent_risk.json");
    let agent_risk_params = AgentRiskParams::load(agent_risk_path)?;

    // Setup terminal
    let mut terminal = tui::setup_terminal()?;

    // Create application state
    let mut app = App::new();
    app.agent_risk_params = agent_risk_params;
    app.authenticated = credentials_valid;

    // Show auth error if credentials were provided but invalid
    if let Some(error) = auth_error {
        app.show_error(format!("Auth failed: {}", error));
    }

    // Create message channel (shared with connection manager).
    // 512 slots: absorbs WebSocket data bursts without blocking producers.
    let (tx, mut rx) = mpsc::channel::<Message>(512);

    // Create command channel for subscription tracking.
    // 32 slots: commands are infrequent (subscribe/unsubscribe/token-used).
    let (cmd_tx, cmd_rx) = mpsc::channel::<ConnectionCommand>(32);

    // Shared writer: connection manager writes, main loop reads
    let writer: Arc<tokio::sync::Mutex<Option<leeson::websocket::WsWriter>>> =
        Arc::new(tokio::sync::Mutex::new(None));

    // Spawn the connection manager — credentials move into the manager,
    // which is the sole owner for the rest of the process lifetime.
    let manager = ConnectionManager::new(
        url,
        tls_config,
        app_config.kraken.api_key,
        app_config.kraken.api_secret,
        tx.clone(),
        writer.clone(),
        cmd_rx,
    );
    tokio::spawn(async move { manager.run().await });

    // Spawn event reader for keyboard input
    tui::event::spawn_event_reader(tx.clone());

    // Spawn tick timer for periodic updates
    tui::event::spawn_tick_timer(tx.clone(), 250);

    // Spawn agent subprocesses
    let mut agents: [Option<AgentHandle>; 3] = [None, None, None];
    match spawn_agent(0, "agents/agent1.py", tx.clone()) {
        Ok(handle) => agents[0] = Some(handle),
        Err(e) => app.show_error(format!("Failed to spawn Agent 1: {e}")),
    }

    // Per-symbol throttle for ticker updates to agents (max once per 5 seconds)
    let mut ticker_last_sent: HashMap<String, Instant> = HashMap::new();
    const TICKER_THROTTLE: Duration = Duration::from_secs(5);

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
            // Intercept token state changes to forward to agents
            let message = match message {
                Message::TokenState(state) => {
                    for handle in agents.iter().flatten() {
                        let _ = handle
                            .commands
                            .send(AgentCommand::TokenState(state.label().to_string()));
                    }
                    tui::event::update(&mut app, Message::TokenState(state));
                    continue;
                }
                other => other,
            };

            // Intercept AgentReady to send risk limits (needs risk_guard access)
            let message = match message {
                Message::AgentReady(agent_index) => {
                    app.add_agent_output(agent_index, "[agent ready]".to_string());
                    if let Some(ref handle) = agents[agent_index] {
                        let mut desc = risk_guard.config().describe_limits();
                        desc.push_str(&app.agent_risk_params.describe());
                        let _ = handle.commands.send(AgentCommand::RiskLimits(desc));
                    }
                    continue;
                }
                other => other,
            };

            // Forward data streams to agents (message passes through to TUI unchanged)
            if let Message::Execution(ref response) = message {
                let cmd = AgentCommand::ExecutionUpdate(response.data.clone());
                for handle in agents.iter().flatten() {
                    let _ = handle.commands.send(cmd.clone());
                }
            }
            if let Message::Balance(ref response) = message {
                let cmd = AgentCommand::BalanceUpdate(response.data.clone());
                for handle in agents.iter().flatten() {
                    let _ = handle.commands.send(cmd.clone());
                }
            }
            if let Message::Trade(ref response) = message {
                let cmd = AgentCommand::TradeUpdate(response.data.clone());
                for handle in agents.iter().flatten() {
                    let _ = handle.commands.send(cmd.clone());
                }
            }
            if let Message::OrderPlaced(ref response) = message {
                let cmd = AgentCommand::OrderResponse {
                    success: response.success,
                    order_id: response.result.as_ref().map(|r| r.order_id.clone()),
                    cl_ord_id: response.result.as_ref().and_then(|r| r.cl_ord_id.clone()),
                    order_userref: response.result.as_ref().and_then(|r| r.order_userref),
                    error: response.error.clone(),
                };
                for handle in agents.iter().flatten() {
                    let _ = handle.commands.send(cmd.clone());
                }
            }
            if let Message::Ticker(ref response) = message {
                let now = Instant::now();
                for data in &response.data {
                    let should_send = ticker_last_sent
                        .get(&data.symbol)
                        .is_none_or(|last| now.duration_since(*last) >= TICKER_THROTTLE);
                    if should_send {
                        ticker_last_sent.insert(data.symbol.clone(), now);
                        let cmd = AgentCommand::TickerUpdate(data.clone());
                        for handle in agents.iter().flatten() {
                            let _ = handle.commands.send(cmd.clone());
                        }
                    }
                }
            }

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
                        if let Err(e) = cmd_tx.try_send(ConnectionCommand::PairSubscribed(symbol)) {
                            tracing::warn!("command channel full, dropping PairSubscribed: {e}");
                        }
                    }
                    tui::event::Action::UnsubscribePair(symbol) => {
                        if let Err(e) = cmd_tx.try_send(ConnectionCommand::PairUnsubscribed(symbol))
                        {
                            tracing::warn!("command channel full, dropping PairUnsubscribed: {e}");
                        }
                    }
                    tui::event::Action::ResyncBook(symbol) => {
                        tracing::info!(symbol = %symbol, "resyncing order book");
                        let mut guard = writer.lock().await;
                        if let Some(ref mut w) = *guard {
                            let symbols = vec![symbol];
                            let _ = unsubscribe(w, &Channel::Book, &symbols, None).await;
                            let _ = subscribe_book(w, &symbols, BookDepth::D25, None).await;
                        }
                    }
                    tui::event::Action::SendToAgent1(message) => {
                        app.add_agent_output(0, format!("You: {message}"));
                        if let Some(ref handle) = agents[0] {
                            let _ = handle.commands.send(AgentCommand::UserMessage(message));
                        } else {
                            app.add_agent_output(0, "[agent not running]".to_string());
                        }
                    }
                    tui::event::Action::SubmitOrder(boxed_params) => {
                        use leeson::risk::RiskVerdict;
                        let params = *boxed_params;
                        let symbol = params.symbol.clone();
                        match risk_guard.check_order(&params) {
                            Ok(RiskVerdict::Approved) => {
                                let request = AddOrderRequest::new(params, None);
                                let mut ws = writer.lock().await;
                                if let Some(ref mut w) = *ws {
                                    let _ = add_order(w, request).await;
                                    risk_guard.record_submission(&symbol);
                                    let _ = cmd_tx.try_send(ConnectionCommand::TokenUsed);
                                }
                            }
                            Ok(RiskVerdict::RequiresConfirmation { reason }) => {
                                app.pending_order = Some(PendingOrder {
                                    params,
                                    reason: reason.clone(),
                                });
                                app.mode = Mode::Confirm;
                                tracing::info!(%reason, "order requires confirmation");
                            }
                            Err(e) => {
                                app.show_error(format!("Order rejected: {e}"));
                                tracing::warn!(%e, "order rejected by risk guard");
                            }
                        }
                    }
                    tui::event::Action::ConfirmOrder => {
                        if let Some(pending) = app.pending_order.take() {
                            let symbol = pending.params.symbol.clone();
                            let request = AddOrderRequest::new(pending.params, None);
                            let mut ws = writer.lock().await;
                            if let Some(ref mut w) = *ws {
                                let _ = add_order(w, request).await;
                                risk_guard.record_submission(&symbol);
                                let _ = cmd_tx.try_send(ConnectionCommand::TokenUsed);
                            }
                        }
                    }
                    tui::event::Action::CancelOrder(_order_id) => {
                        // TODO: Implement order cancellation
                    }
                    tui::event::Action::SaveRiskParams(params) => {
                        if let Err(e) = params.save(agent_risk_path) {
                            app.show_error(format!("Failed to save risk params: {e}"));
                        }
                        // Re-send combined limits to all running agents
                        let mut desc = risk_guard.config().describe_limits();
                        desc.push_str(&params.describe());
                        for handle in agents.iter().flatten() {
                            let _ = handle.commands.send(AgentCommand::RiskLimits(desc.clone()));
                        }
                    }
                }
            }
        }
    }

    // Shut down running agents
    for handle in agents.iter().flatten() {
        let _ = handle.commands.send(AgentCommand::Shutdown);
    }

    // Restore terminal
    tui::restore_terminal(&mut terminal)?;

    Ok(())
}
