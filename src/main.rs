use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use leeson::LeesonError;
use leeson::agent::{AgentCommand, AgentHandle, spawn_multi_agent};
use leeson::auth::validate_credentials;
use leeson::config::fetch_config;
use leeson::credentials::{self, CredentialKey};
use leeson::models::Channel;
use leeson::models::add_order::AddOrderRequest;
use leeson::models::book::BookDepth;
use leeson::risk::RiskGuard;
use leeson::risk::config::{AgentRiskParams, RiskConfig};
use leeson::simulation::SimulationEngine;
use leeson::tls::build_tls_config;
use leeson::tui::app::{ApiKeysEditState, Mode, PendingOrder, SimulationStats};
use leeson::tui::{self, App, Message};
use leeson::websocket::{
    ConnectionCommand, ConnectionManager, add_order, subscribe, subscribe_book, unsubscribe,
};

#[tokio::main]
async fn main() -> Result<(), LeesonError> {
    credentials::populate_env_from_keychain();
    let app_config = fetch_config()?;
    let tls_config = Arc::new(build_tls_config()?);

    // Load risk configuration (required — running without risk limits is a hard error)
    let risk_config = RiskConfig::load(Path::new("risk.json"))?;
    let mut risk_guard = RiskGuard::new(risk_config);

    let mut sim_engine: Option<SimulationEngine> = if app_config.simulation {
        Some(SimulationEngine::new())
    } else {
        None
    };

    let any_credentials_missing = CredentialKey::ALL
        .iter()
        .filter(|key| key.required())
        .any(|key| std::env::var(key.env_var()).unwrap_or_default().is_empty());

    let has_credentials = matches!(
        (&app_config.kraken.api_key, &app_config.kraken.api_secret),
        (Some(k), Some(s)) if !k.is_empty() && !s.is_empty()
    );

    // Validate API credentials only when all creds are present;
    // when some are missing the overlay will open first.
    let (credentials_valid, auth_error) = if !any_credentials_missing && has_credentials {
        let key = app_config.kraken.api_key.as_deref().unwrap();
        let secret = app_config.kraken.api_secret.as_deref().unwrap();
        match validate_credentials(key, secret, (*tls_config).clone()).await {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        }
    } else {
        (false, None)
    };

    // Use authenticated endpoint if credentials are valid, otherwise use configured URL.
    // In simulation mode, always use the public endpoint (no auth needed for data-only).
    let url = if app_config.simulation {
        app_config.kraken.websocket_url.clone()
    } else if credentials_valid {
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
    app.simulation = app_config.simulation;
    app.token_usage.input_cost_per_million = app_config.token_input_cost;
    app.token_usage.output_cost_per_million = app_config.token_output_cost;

    // Show auth error if credentials were provided but invalid
    if let Some(error) = auth_error {
        app.show_error(format!("Auth failed: {}", error));
    }

    // Auto-open the API keys overlay when any credentials are missing
    if any_credentials_missing {
        app.api_keys_edit = Some(ApiKeysEditState::new());
        app.mode = Mode::ApiKeys;
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

    // Track whether the connection manager and agents have been spawned.
    // When credentials are missing we defer spawning until the overlay is dismissed.
    let mut setup_complete = !any_credentials_missing;
    let mut deferred_cmd_rx: Option<mpsc::Receiver<ConnectionCommand>> = None;

    if setup_complete {
        // Spawn the connection manager — credentials move into the manager,
        // which is the sole owner for the rest of the process lifetime.
        let manager = ConnectionManager::new(
            url,
            tls_config.clone(),
            app_config.kraken.api_key,
            app_config.kraken.api_secret,
            tx.clone(),
            writer.clone(),
            cmd_rx,
        );
        tokio::spawn(async move { manager.run().await });
    } else {
        deferred_cmd_rx = Some(cmd_rx);
    }

    // Spawn event reader for keyboard input
    tui::event::spawn_event_reader(tx.clone());

    // Spawn tick timer for periodic updates
    tui::event::spawn_tick_timer(tx.clone(), 250);

    // Spawn agent subprocesses (deferred when credentials are missing)
    let mut agents: [Option<AgentHandle>; 3] = [None, None, None];
    if setup_complete {
        match spawn_multi_agent(0, tx.clone()) {
            Ok(handle) => agents[0] = Some(handle),
            Err(e) => app.show_error(format!("Failed to spawn multi-agent system: {e}")),
        }
    }

    // Per-symbol throttle for ticker updates to agents (max once per 5 seconds)
    let mut ticker_last_sent: HashMap<String, Instant> = HashMap::new();
    const TICKER_THROTTLE: Duration = Duration::from_secs(5);

    // Main event loop
    loop {
        // Snapshot simulation stats before rendering
        if let Some(ref sim) = sim_engine {
            app.sim_stats = SimulationStats {
                realized_pnl: sim.realized_pnl(),
                unrealized_pnl: sim.unrealized_pnl(&app.tickers),
                trade_count: sim.trade_count(),
                positions: sim.positions().clone(),
                avg_entry_prices: sim.avg_entry_prices().clone(),
                session_secs: sim.session_secs(),
            };
        }

        // Render UI
        terminal
            .draw(|frame| tui::render(frame, &mut app))
            .map_err(|e| LeesonError::Io(e.to_string()))?;

        // Check if we should quit
        if app.should_quit {
            break;
        }

        // Wait for next message
        if let Some(message) = rx.recv().await {
            // Intercept token state changes to forward to agents.
            // In simulation mode, always tell agents the token is valid so
            // they believe they can submit orders.
            let message = match message {
                Message::TokenState(state) => {
                    let label = if sim_engine.is_some() {
                        "valid"
                    } else {
                        state.label()
                    };
                    for handle in agents.iter().flatten() {
                        let _ = handle
                            .commands
                            .send(AgentCommand::TokenState(label.to_string()));
                    }
                    tui::event::update(&mut app, Message::TokenState(state));
                    continue;
                }
                other => other,
            };

            // Intercept AgentReady to send risk limits and active pairs
            let message = match message {
                Message::AgentReady(agent_index) => {
                    app.add_agent_output(agent_index, "[agent ready]".to_string());
                    if let Some(ref handle) = agents[agent_index] {
                        let mut desc = risk_guard.config().describe_limits();
                        desc.push_str(&app.agent_risk_params.describe());
                        let _ = handle.commands.send(AgentCommand::RiskLimits(desc));
                        if !app.selected_pairs.is_empty() {
                            let _ = handle
                                .commands
                                .send(AgentCommand::ActivePairs(app.selected_pairs.clone()));
                        }
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
                        // Forward updated active pairs to all agents
                        let cmd = AgentCommand::ActivePairs(app.selected_pairs.clone());
                        for handle in agents.iter().flatten() {
                            let _ = handle.commands.send(cmd.clone());
                        }
                    }
                    tui::event::Action::UnsubscribePair(symbol) => {
                        if let Err(e) = cmd_tx.try_send(ConnectionCommand::PairUnsubscribed(symbol))
                        {
                            tracing::warn!("command channel full, dropping PairUnsubscribed: {e}");
                        }
                        // Forward updated active pairs to all agents
                        let cmd = AgentCommand::ActivePairs(app.selected_pairs.clone());
                        for handle in agents.iter().flatten() {
                            let _ = handle.commands.send(cmd.clone());
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
                                if let Some(ref mut sim) = sim_engine {
                                    let ticker = app.tickers.get(&symbol);
                                    let (order_resp, exec_resp) =
                                        sim.execute_order(&params, ticker);
                                    // Forward synthesized responses to agents
                                    let cmd = AgentCommand::OrderResponse {
                                        success: order_resp.success,
                                        order_id: order_resp
                                            .result
                                            .as_ref()
                                            .map(|r| r.order_id.clone()),
                                        cl_ord_id: order_resp
                                            .result
                                            .as_ref()
                                            .and_then(|r| r.cl_ord_id.clone()),
                                        order_userref: order_resp
                                            .result
                                            .as_ref()
                                            .and_then(|r| r.order_userref),
                                        error: order_resp.error.clone(),
                                    };
                                    for handle in agents.iter().flatten() {
                                        let _ = handle.commands.send(cmd.clone());
                                    }
                                    if let Some(ref exec) = exec_resp {
                                        let exec_cmd =
                                            AgentCommand::ExecutionUpdate(exec.data.clone());
                                        for handle in agents.iter().flatten() {
                                            let _ = handle.commands.send(exec_cmd.clone());
                                        }
                                    }
                                    // Feed through TUI state update
                                    tui::event::update(&mut app, Message::OrderPlaced(order_resp));
                                    if let Some(exec) = exec_resp {
                                        tui::event::update(&mut app, Message::Execution(exec));
                                    }
                                    risk_guard.record_submission(&symbol);
                                } else {
                                    let request = AddOrderRequest::new(params, None);
                                    let mut ws = writer.lock().await;
                                    if let Some(ref mut w) = *ws {
                                        let _ = add_order(w, request).await;
                                        risk_guard.record_submission(&symbol);
                                        let _ = cmd_tx.try_send(ConnectionCommand::TokenUsed);
                                    }
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
                            if let Some(ref mut sim) = sim_engine {
                                let ticker = app.tickers.get(&symbol);
                                let (order_resp, exec_resp) =
                                    sim.execute_order(&pending.params, ticker);
                                let cmd = AgentCommand::OrderResponse {
                                    success: order_resp.success,
                                    order_id: order_resp
                                        .result
                                        .as_ref()
                                        .map(|r| r.order_id.clone()),
                                    cl_ord_id: order_resp
                                        .result
                                        .as_ref()
                                        .and_then(|r| r.cl_ord_id.clone()),
                                    order_userref: order_resp
                                        .result
                                        .as_ref()
                                        .and_then(|r| r.order_userref),
                                    error: order_resp.error.clone(),
                                };
                                for handle in agents.iter().flatten() {
                                    let _ = handle.commands.send(cmd.clone());
                                }
                                if let Some(ref exec) = exec_resp {
                                    let exec_cmd = AgentCommand::ExecutionUpdate(exec.data.clone());
                                    for handle in agents.iter().flatten() {
                                        let _ = handle.commands.send(exec_cmd.clone());
                                    }
                                }
                                tui::event::update(&mut app, Message::OrderPlaced(order_resp));
                                if let Some(exec) = exec_resp {
                                    tui::event::update(&mut app, Message::Execution(exec));
                                }
                                risk_guard.record_submission(&symbol);
                            } else {
                                let request = AddOrderRequest::new(pending.params, None);
                                let mut ws = writer.lock().await;
                                if let Some(ref mut w) = *ws {
                                    let _ = add_order(w, request).await;
                                    risk_guard.record_submission(&symbol);
                                    let _ = cmd_tx.try_send(ConnectionCommand::TokenUsed);
                                }
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
                    tui::event::Action::SaveApiKeys { values } => {
                        let mut saved = 0u32;
                        let mut errors = Vec::new();
                        let mut kraken_changed = false;

                        for (i, new_value) in values.into_iter().enumerate() {
                            if let Some(value) = new_value {
                                let key = CredentialKey::ALL[i];
                                match credentials::save(key, &value) {
                                    Ok(()) => {
                                        // SAFETY: main thread, single writer
                                        unsafe {
                                            std::env::set_var(key.env_var(), &value);
                                        }
                                        saved += 1;
                                        if i == 1 || i == 2 {
                                            kraken_changed = true;
                                        }
                                    }
                                    Err(e) => {
                                        errors.push(format!("{}: {e}", key.label()));
                                    }
                                }
                            }
                        }

                        if !errors.is_empty() {
                            app.show_error(format!("Keychain errors: {}", errors.join(", ")));
                        } else if saved > 0 {
                            app.show_error(format!("{saved} API key(s) saved"));
                        }

                        // If Kraken credentials changed, tell the connection manager
                        // (only if it has already been spawned — otherwise it will
                        // receive the credentials at construction time).
                        if kraken_changed && setup_complete {
                            let api_key = credentials::load(CredentialKey::KrakenApiKey);
                            let api_secret = credentials::load(CredentialKey::KrakenApiSecret);
                            if let Err(e) = cmd_tx.try_send(ConnectionCommand::UpdateCredentials {
                                api_key,
                                api_secret,
                            }) {
                                tracing::warn!(
                                    "command channel full, dropping UpdateCredentials: {e}"
                                );
                            }
                            app.authenticated = true;
                        }
                    }
                }
            }

            // Complete deferred setup once the API keys overlay is dismissed
            if !setup_complete && app.mode != Mode::ApiKeys {
                setup_complete = true;

                // Re-read credentials (SaveApiKeys updated env vars and keychain)
                let api_key = credentials::load(CredentialKey::KrakenApiKey);
                let api_secret = credentials::load(CredentialKey::KrakenApiSecret);

                let has_creds = api_key.is_some() && api_secret.is_some();

                // Determine the WebSocket URL based on available credentials
                let url = if app_config.simulation {
                    app_config.kraken.websocket_url.clone()
                } else if has_creds {
                    // Validate credentials before connecting
                    if let (Some(k), Some(s)) = (&api_key, &api_secret) {
                        match validate_credentials(k, s, (*tls_config).clone()).await {
                            Ok(_) => app.authenticated = true,
                            Err(e) => {
                                app.show_error(format!("Auth failed: {e}"));
                            }
                        }
                    }
                    if app.authenticated {
                        "wss://ws-auth.kraken.com/v2".to_string()
                    } else {
                        app_config.kraken.websocket_url.clone()
                    }
                } else {
                    app_config.kraken.websocket_url.clone()
                };

                // Spawn the connection manager with the deferred cmd_rx
                if let Some(cmd_rx) = deferred_cmd_rx.take() {
                    let manager = ConnectionManager::new(
                        url,
                        tls_config.clone(),
                        api_key,
                        api_secret,
                        tx.clone(),
                        writer.clone(),
                        cmd_rx,
                    );
                    tokio::spawn(async move { manager.run().await });
                }

                // Spawn agent subprocesses
                match spawn_multi_agent(0, tx.clone()) {
                    Ok(handle) => agents[0] = Some(handle),
                    Err(e) => app.show_error(format!("Failed to spawn multi-agent system: {e}")),
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
