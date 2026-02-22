//! Event handling for the TUI.

use std::time::{Duration, Instant};

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use crate::models::add_order::AddOrderParams;
use crate::models::balance::BalanceResponse;
use crate::models::book::{BookUpdateResponse, calculate_checksum};
use crate::models::candle::CandleUpdateResponse;
use crate::models::execution::ExecutionUpdateResponse;
use crate::models::ticker::TickerUpdateResponse;
use crate::models::trade::TradeUpdateResponse;
use crate::models::{
    AddOrderResponse, AmendOrderResponse, CancelAllResponse, CancelOrderResponse,
    StatusUpdateResponse,
};

use crate::risk::config::AgentRiskParams;

use super::app::{
    ApiKeysEditState, App, AssetBalance, Focus, MAX_BOOK_DEPTH, MAX_ORDERBOOK_HISTORY, Mode,
    OrderBookSnapshot, RiskEditState, Tab,
};

/// Maximum length (in bytes) for agent input text.
///
/// Prevents unbounded memory growth in the input buffer and guards against
/// oversized payloads being serialized to agent stdin. 4 KiB is generous
/// for a single operator command.
const MAX_INPUT_LENGTH: usize = 4096;

/// Maximum open orders tracked per symbol before oldest are dropped.
const MAX_OPEN_ORDERS_PER_SYMBOL: usize = 200;

/// Events that can occur in the application.
#[derive(Debug)]
pub enum Event {
    /// A key was pressed.
    Key(KeyEvent),
    /// Terminal was resized.
    Resize(u16, u16),
    /// Periodic tick for UI updates.
    Tick,
}

/// Messages that update application state.
#[derive(Debug)]
pub enum Message {
    /// Input event from terminal.
    Input(Event),

    /// Ticker update from WebSocket.
    Ticker(TickerUpdateResponse),
    /// Order book update from WebSocket.
    Book(BookUpdateResponse),
    /// Trade update from WebSocket.
    Trade(TradeUpdateResponse),
    /// Candle update from WebSocket.
    Candle(CandleUpdateResponse),
    /// Execution update from WebSocket.
    Execution(ExecutionUpdateResponse),
    /// Balance update from WebSocket.
    Balance(BalanceResponse),
    /// Status update from WebSocket.
    Status(StatusUpdateResponse),
    /// Heartbeat received.
    Heartbeat,

    /// Order placement response.
    OrderPlaced(AddOrderResponse),
    /// Order cancellation response.
    OrderCancelled(CancelOrderResponse),
    /// Order amendment response.
    OrderAmended(AmendOrderResponse),
    /// Cancel all response.
    AllOrdersCancelled(CancelAllResponse),

    /// WebSocket connected.
    Connected,
    /// WebSocket disconnected.
    Disconnected,
    /// WebSocket reconnecting.
    Reconnecting,

    /// Output line from an agent subprocess.
    AgentOutput { agent_index: usize, line: String },
    /// Agent subprocess signaled readiness.
    AgentReady(usize),
    /// Agent subprocess exited.
    AgentExited {
        agent_index: usize,
        error: Option<String>,
    },

    /// Agent requested an order placement.
    AgentOrderRequest {
        agent_index: usize,
        symbol: String,
        side: String,
        order_type: String,
        qty: String,
        price: Option<String>,
        cl_ord_id: Option<String>,
    },

    /// Cumulative token usage from agent LLM calls.
    AgentTokenUsage {
        input_tokens: u64,
        output_tokens: u64,
    },

    /// Token lifecycle state change.
    TokenState(super::app::TokenState),

    /// Private (authenticated) WebSocket channel connected or lost.
    PrivateChannelStatus(bool),

    /// Request to quit the application.
    Quit,
}

/// Spawns a task that polls for terminal events and sends them to a channel.
pub fn spawn_event_reader(tx: mpsc::Sender<Message>) {
    tokio::spawn(async move {
        loop {
            // Poll for events with a 50ms timeout
            match tokio::task::spawn_blocking(|| {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    event::read().ok()
                } else {
                    None
                }
            })
            .await
            {
                Ok(Some(CrosstermEvent::Key(key))) => {
                    match tx.try_send(Message::Input(Event::Key(key))) {
                        Ok(()) => {}
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            tracing::warn!("message channel full, dropping key event");
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => break,
                    }
                }
                Ok(Some(CrosstermEvent::Resize(w, h))) => {
                    match tx.try_send(Message::Input(Event::Resize(w, h))) {
                        Ok(()) => {}
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            tracing::warn!("message channel full, dropping resize event");
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => break,
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
}

/// Spawns a task that sends periodic tick events.
pub fn spawn_tick_timer(tx: mpsc::Sender<Message>, interval_ms: u64) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
        loop {
            interval.tick().await;
            match tx.try_send(Message::Input(Event::Tick)) {
                Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => {
                    // Ticks are periodic; dropping one is harmless.
                }
                Err(mpsc::error::TrySendError::Closed(_)) => break,
            }
        }
    });
}

/// Updates application state based on a message.
pub fn update(app: &mut App, message: Message) -> Option<Action> {
    match message {
        Message::Input(event) => handle_input(app, event),
        Message::Ticker(response) => {
            for data in response.data {
                app.update_ticker(data.symbol.clone(), data);
            }
            None
        }
        Message::Book(response) => {
            const MAX_CHECKSUM_FAILURES: u8 = 3;
            const RESYNC_COOLDOWN: Duration = Duration::from_secs(5);

            let is_snapshot = response.tpe == "snapshot";
            let mut resync_action: Option<Action> = None;

            for data in response.data {
                let symbol = data.symbol.clone();
                let expected_checksum = data.checksum;
                let state = app.orderbooks.entry(symbol.clone()).or_default();

                if is_snapshot {
                    // Snapshot: replace entire book and reset staleness
                    state.bids = data.bids;
                    state.asks = data.asks;
                    state.is_stale = false;
                    state.checksum_failures = 0;
                    state.last_resync_request = None;
                } else {
                    // Update: apply incremental changes
                    // A level with qty=0 means remove that price level
                    for level in data.bids {
                        if level.qty == rust_decimal::Decimal::ZERO {
                            state.bids.retain(|b| b.price != level.price);
                        } else if let Some(existing) =
                            state.bids.iter_mut().find(|b| b.price == level.price)
                        {
                            existing.qty = level.qty;
                        } else {
                            // Binary search for insertion point (bids sorted descending)
                            let pos = state
                                .bids
                                .binary_search_by(|probe| level.price.cmp(&probe.price))
                                .unwrap_or_else(|pos| pos);
                            state.bids.insert(pos, level);
                        }
                    }
                    for level in data.asks {
                        if level.qty == rust_decimal::Decimal::ZERO {
                            state.asks.retain(|a| a.price != level.price);
                        } else if let Some(existing) =
                            state.asks.iter_mut().find(|a| a.price == level.price)
                        {
                            existing.qty = level.qty;
                        } else {
                            // Binary search for insertion point (asks sorted ascending)
                            let pos = state
                                .asks
                                .binary_search_by(|probe| probe.price.cmp(&level.price))
                                .unwrap_or_else(|pos| pos);
                            state.asks.insert(pos, level);
                        }
                    }

                    // Verify checksum after applying incremental update
                    let local_checksum = calculate_checksum(&state.asks, &state.bids);
                    if local_checksum != expected_checksum {
                        state.is_stale = true;
                        state.checksum_failures = state.checksum_failures.saturating_add(1);
                        tracing::warn!(
                            symbol = %symbol,
                            expected = expected_checksum,
                            calculated = local_checksum,
                            failures = state.checksum_failures,
                            "order book checksum mismatch"
                        );

                        let cooldown_elapsed = state
                            .last_resync_request
                            .is_none_or(|t| t.elapsed() >= RESYNC_COOLDOWN);

                        if state.checksum_failures <= MAX_CHECKSUM_FAILURES
                            && cooldown_elapsed
                            && resync_action.is_none()
                        {
                            state.last_resync_request = Some(Instant::now());
                            resync_action = Some(Action::ResyncBook(symbol.clone()));
                        }
                    } else {
                        state.is_stale = false;
                        state.checksum_failures = 0;
                    }
                }

                // Enforce depth limit
                state.bids.truncate(MAX_BOOK_DEPTH);
                state.asks.truncate(MAX_BOOK_DEPTH);

                state.checksum = expected_checksum;
                state.last_update = Some(Instant::now());

                // Capture snapshot for history after updating state
                if let (Some(best_bid), Some(best_ask)) = (state.bids.first(), state.asks.first()) {
                    let snapshot = OrderBookSnapshot {
                        timestamp: data.timestamp.clone(),
                        best_bid: best_bid.price,
                        best_ask: best_ask.price,
                        spread: best_ask.price - best_bid.price,
                    };
                    if state.history.len() >= MAX_ORDERBOOK_HISTORY {
                        state.history.pop_front();
                    }
                    state.history.push_back(snapshot);
                }
            }
            resync_action
        }
        Message::Trade(response) => {
            for data in response.data {
                let symbol = data.symbol.clone();
                app.add_trade(&symbol, data);
            }
            None
        }
        Message::Candle(response) => {
            for data in response.data {
                let candles = app
                    .candles
                    .entry(data.symbol.clone())
                    .or_insert_with(|| std::collections::VecDeque::with_capacity(100));
                if candles.len() >= 100 {
                    candles.pop_front();
                }
                candles.push_back(data);
            }
            None
        }
        Message::Execution(response) => {
            for data in response.data {
                // Add to open or executed orders based on status
                let symbol = data.symbol.clone();
                match data.order_status.as_str() {
                    "open" | "pending" | "new" => {
                        let orders = app.open_orders.entry(symbol).or_default();
                        // Update or add
                        if let Some(pos) = orders.iter().position(|o| o.order_id == data.order_id) {
                            orders[pos] = data;
                        } else {
                            if orders.len() >= MAX_OPEN_ORDERS_PER_SYMBOL {
                                orders.remove(0);
                            }
                            orders.push(data);
                        }
                    }
                    "filled" | "canceled" | "expired" => {
                        // Remove from open orders
                        if let Some(orders) = app.open_orders.get_mut(&symbol) {
                            orders.retain(|o| o.order_id != data.order_id);
                        }
                        // Add to executed orders
                        let executed = app
                            .executed_orders
                            .entry(symbol)
                            .or_insert_with(|| std::collections::VecDeque::with_capacity(100));
                        if executed.len() >= 100 {
                            executed.pop_front();
                        }
                        executed.push_back(data);
                    }
                    _ => {}
                }
            }
            None
        }
        Message::Balance(response) => {
            // Process balance snapshot or update
            for data in response.data {
                let mut spot = rust_decimal::Decimal::ZERO;
                let mut earn = rust_decimal::Decimal::ZERO;

                for wallet in &data.wallets {
                    match wallet.wallet_type.as_str() {
                        "spot" => spot = wallet.balance,
                        "earn" => earn = wallet.balance,
                        _ => {}
                    }
                }

                let balance = AssetBalance {
                    asset: data.asset.clone(),
                    total: data.balance,
                    spot,
                    earn,
                };
                app.asset_balances.insert(data.asset, balance);
            }

            // Update USD balance field for compatibility
            if let Some(usd) = app.asset_balances.get("USD") {
                app.balance = usd.total;
            }
            None
        }
        Message::Status(_) => {
            app.connection_status = super::app::ConnectionStatus::Connected;
            None
        }
        Message::Heartbeat => {
            app.last_heartbeat = Some(std::time::Instant::now());
            None
        }
        Message::OrderPlaced(response) => {
            if !response.success
                && let Some(error) = response.error
            {
                app.show_error(error);
            }
            None
        }
        Message::OrderCancelled(response) => {
            if !response.success
                && let Some(error) = response.error
            {
                app.show_error(error);
            }
            None
        }
        Message::OrderAmended(response) => {
            if !response.success
                && let Some(error) = response.error
            {
                app.show_error(error);
            }
            None
        }
        Message::AllOrdersCancelled(_) => None,
        Message::Connected => {
            app.connection_status = super::app::ConnectionStatus::Connected;
            None
        }
        Message::Disconnected => {
            app.connection_status = super::app::ConnectionStatus::Disconnected;
            app.private_connected = false;
            None
        }
        Message::Reconnecting => {
            app.connection_status = super::app::ConnectionStatus::Reconnecting;
            app.private_connected = false;
            None
        }
        Message::PrivateChannelStatus(connected) => {
            app.private_connected = connected;
            None
        }
        Message::AgentOutput { agent_index, line } => {
            app.add_agent_output(agent_index, line);
            None
        }
        Message::AgentReady(agent_index) => {
            app.add_agent_output(agent_index, "[agent ready]".to_string());
            None
        }
        Message::AgentOrderRequest {
            agent_index,
            symbol,
            side,
            order_type,
            qty,
            price,
            cl_ord_id,
        } => {
            use crate::models::add_order::{AddOrderBuilder, OrderSide};
            use rust_decimal::Decimal;
            use std::str::FromStr;

            let parse_result = (|| -> Result<AddOrderParams, String> {
                let side = match side.to_lowercase().as_str() {
                    "buy" => OrderSide::Buy,
                    "sell" => OrderSide::Sell,
                    other => return Err(format!("invalid side: {other}")),
                };
                let qty = Decimal::from_str(&qty).map_err(|e| format!("invalid qty: {e}"))?;

                let mut builder = match order_type.to_lowercase().as_str() {
                    "market" => AddOrderBuilder::market(side, &symbol, qty),
                    "limit" => {
                        let price_str = price.as_deref().ok_or("limit order requires a price")?;
                        let price = Decimal::from_str(price_str)
                            .map_err(|e| format!("invalid price: {e}"))?;
                        AddOrderBuilder::limit(side, &symbol, qty, price)
                    }
                    other => return Err(format!("unsupported order type: {other}")),
                };

                if let Some(ref id) = cl_ord_id {
                    builder = builder.with_cl_ord_id(id);
                }

                // Use a placeholder token — real token is set in main.rs before submission
                builder
                    .build("pending")
                    .map_err(|e| format!("order validation failed: {e}"))
            })();

            match parse_result {
                Ok(params) => {
                    app.add_agent_output(
                        agent_index,
                        format!(
                            "[order] {:?} {:?} {} {} @ {}",
                            params.side,
                            params.order_type,
                            params.order_qty,
                            params.symbol,
                            params
                                .limit_price
                                .map_or("market".to_string(), |p| p.to_string())
                        ),
                    );
                    return Some(Action::SubmitOrder(Box::new(params)));
                }
                Err(e) => {
                    app.add_agent_output(agent_index, format!("[order error] {e}"));
                    app.show_error(format!("Agent order error: {e}"));
                }
            }
            None
        }
        Message::AgentExited { agent_index, error } => {
            let msg = match error {
                Some(e) => format!("[agent exited: {e}]"),
                None => "[agent exited]".to_string(),
            };
            app.add_agent_output(agent_index, msg);
            None
        }
        Message::AgentTokenUsage {
            input_tokens,
            output_tokens,
        } => {
            app.token_usage.input_tokens = input_tokens;
            app.token_usage.output_tokens = output_tokens;
            None
        }
        Message::TokenState(state) => {
            app.token_state = state;
            None
        }
        Message::Quit => {
            app.should_quit = true;
            None
        }
    }
}

/// Actions that require external handling (e.g., sending WebSocket messages).
#[derive(Debug)]
pub enum Action {
    /// Send a message to Agent 1.
    SendToAgent1(String),
    /// Subscribe to a trading pair.
    SubscribePair(String),
    /// Unsubscribe from a trading pair.
    UnsubscribePair(String),
    /// Re-subscribe to a pair's book channel after checksum mismatch.
    ResyncBook(String),
    /// Submit an order (validated by risk guard before sending).
    SubmitOrder(Box<AddOrderParams>),
    /// Operator confirmed a pending order.
    ConfirmOrder,
    /// Cancel an order.
    CancelOrder(String),
    /// Operator saved updated agent risk parameters.
    SaveRiskParams(AgentRiskParams),
    /// Operator saved API keys from the overlay.
    SaveApiKeys {
        /// New values for each credential (None = unchanged).
        values: [Option<String>; 3],
    },
}

/// Handles input events and updates application state.
fn handle_input(app: &mut App, event: Event) -> Option<Action> {
    match event {
        Event::Key(key) => handle_key(app, key),
        Event::Resize(_, _) => None,
        Event::Tick => {
            app.clear_stale_errors();
            None
        }
    }
}

/// Handles key press events.
fn handle_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    // RiskEdit mode handles its own Esc (two-stage: cancel edit, then close)
    if app.mode == Mode::RiskEdit {
        return handle_risk_edit_mode(app, key);
    }

    // ApiKeys mode handles its own Esc (two-stage: cancel edit, then close)
    if app.mode == Mode::ApiKeys {
        return handle_api_keys_mode(app, key);
    }

    // Global keys (work in any mode)
    match key.code {
        KeyCode::Char('q') if key.modifiers.is_empty() && app.mode == Mode::Normal => {
            app.should_quit = true;
            return None;
        }
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            return None;
        }
        _ => {}
    }

    // Mode-specific handling
    match app.mode {
        Mode::Normal => handle_normal_mode(app, key),
        Mode::Insert => handle_insert_mode(app, key),
        Mode::Confirm => handle_confirm_mode(app, key),
        Mode::RiskEdit | Mode::ApiKeys => unreachable!(),
    }
}

/// Handles keys in normal mode.
fn handle_normal_mode(app: &mut App, key: KeyEvent) -> Option<Action> {
    match key.code {
        // Tab navigation
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.previous_tab();
            } else {
                app.next_tab();
            }
            None
        }
        KeyCode::BackTab => {
            app.previous_tab();
            None
        }

        // Help
        KeyCode::Char('?') => {
            // TODO: Toggle help overlay
            None
        }

        // Risk parameters overlay
        KeyCode::Char('r') => {
            app.risk_edit = Some(RiskEditState::new(&app.agent_risk_params));
            app.mode = Mode::RiskEdit;
            None
        }

        // API keys overlay
        KeyCode::Char('a') => {
            app.api_keys_edit = Some(ApiKeysEditState::new());
            app.mode = Mode::ApiKeys;
            None
        }

        _ => {
            // Delegate to tab-specific handling
            match app.current_tab().clone() {
                Tab::Agent => handle_agent_tab_keys(app, key),
                Tab::TradingPair(symbol) => handle_trading_pair_tab_keys(app, key, &symbol),
            }
        }
    }
}

/// Handles keys for the Agent tab.
fn handle_agent_tab_keys(app: &mut App, key: KeyEvent) -> Option<Action> {
    match key.code {
        // Focus navigation
        KeyCode::Char('1') => {
            app.focus = Focus::AgentOutput1;
            None
        }
        KeyCode::Char('2') => {
            app.focus = Focus::AgentOutput2;
            None
        }
        KeyCode::Char('3') => {
            app.focus = Focus::AgentOutput3;
            None
        }

        // Pair selector navigation
        KeyCode::Char('j') | KeyCode::Down => {
            if app.focus == Focus::PairSelector
                && app.pair_selector_index < app.available_pairs.len().saturating_sub(1)
            {
                app.pair_selector_index += 1;
            }
            None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.focus == Focus::PairSelector {
                app.pair_selector_index = app.pair_selector_index.saturating_sub(1);
            }
            None
        }

        // Toggle pair selection
        KeyCode::Char(' ') => {
            if app.focus == Focus::PairSelector
                && let Some(symbol) = app.available_pairs.get(app.pair_selector_index).cloned()
            {
                let was_selected = app.is_pair_selected(&symbol);
                app.toggle_pair(&symbol);
                return if was_selected {
                    Some(Action::UnsubscribePair(symbol))
                } else {
                    Some(Action::SubscribePair(symbol))
                };
            }
            None
        }

        // Enter insert mode for agent input
        KeyCode::Char('i') | KeyCode::Enter => {
            if app.focus == Focus::AgentInput {
                app.mode = Mode::Insert;
            }
            None
        }

        // Focus switching
        KeyCode::Char('h') | KeyCode::Left => {
            app.focus = match app.focus {
                Focus::AgentOutput2 => Focus::AgentOutput1,
                Focus::AgentOutput3 => Focus::AgentOutput2,
                Focus::PairSelector => Focus::AgentInput,
                _ => app.focus,
            };
            None
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.focus = match app.focus {
                Focus::AgentOutput1 => Focus::AgentOutput2,
                Focus::AgentOutput2 => Focus::AgentOutput3,
                Focus::AgentInput => Focus::PairSelector,
                _ => app.focus,
            };
            None
        }

        _ => None,
    }
}

/// Handles keys for trading pair tabs.
fn handle_trading_pair_tab_keys(app: &mut App, key: KeyEvent, _symbol: &str) -> Option<Action> {
    match key.code {
        // Panel focus navigation
        KeyCode::Char('h') | KeyCode::Left => {
            app.focus = match app.focus {
                Focus::Chart => Focus::OrderBook,
                Focus::Orders => Focus::Trades,
                _ => app.focus,
            };
            None
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.focus = match app.focus {
                Focus::OrderBook => Focus::Chart,
                Focus::Trades => Focus::Orders,
                _ => app.focus,
            };
            None
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.focus = match app.focus {
                Focus::OrderBook => Focus::Trades,
                Focus::Chart => Focus::Orders,
                _ => app.focus,
            };
            None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.focus = match app.focus {
                Focus::Trades => Focus::OrderBook,
                Focus::Orders => Focus::Chart,
                _ => app.focus,
            };
            None
        }

        // Toggle chart type
        KeyCode::Char('g') => {
            app.chart_type.toggle();
            None
        }

        // Toggle orders view
        KeyCode::Char('o') => {
            app.orders_view.toggle();
            None
        }

        // Timeframe shortcuts
        KeyCode::Char('1') => {
            app.chart_timeframe = super::app::Timeframe::M1;
            None
        }
        KeyCode::Char('2') => {
            app.chart_timeframe = super::app::Timeframe::M5;
            None
        }
        KeyCode::Char('3') => {
            app.chart_timeframe = super::app::Timeframe::M15;
            None
        }
        KeyCode::Char('4') => {
            app.chart_timeframe = super::app::Timeframe::H1;
            None
        }
        KeyCode::Char('5') => {
            app.chart_timeframe = super::app::Timeframe::H4;
            None
        }
        KeyCode::Char('6') => {
            app.chart_timeframe = super::app::Timeframe::D1;
            None
        }

        // New order
        KeyCode::Char('n') => {
            // TODO: Open order form modal
            None
        }

        // Cancel order
        KeyCode::Char('c') => {
            // TODO: Cancel selected order
            None
        }

        // Edit order
        KeyCode::Char('e') => {
            // TODO: Edit selected order
            None
        }

        _ => None,
    }
}

/// Handles keys in insert mode (text input).
fn handle_insert_mode(app: &mut App, key: KeyEvent) -> Option<Action> {
    if app.focus != Focus::AgentInput {
        return None;
    }

    match key.code {
        KeyCode::Enter => {
            let command = sanitize_input(&std::mem::take(&mut app.agent_input));
            app.agent_input_cursor = 0;
            app.mode = Mode::Normal;
            if !command.is_empty() {
                return Some(Action::SendToAgent1(command));
            }
            None
        }
        KeyCode::Char(c) => {
            if c.is_control() {
                return None;
            }
            if app.agent_input.len() + c.len_utf8() > MAX_INPUT_LENGTH {
                return None;
            }
            app.agent_input.insert(app.agent_input_cursor, c);
            app.agent_input_cursor += c.len_utf8();
            None
        }
        KeyCode::Backspace => {
            if app.agent_input_cursor > 0 {
                app.agent_input_cursor -= 1;
                app.agent_input.remove(app.agent_input_cursor);
            }
            None
        }
        KeyCode::Delete => {
            if app.agent_input_cursor < app.agent_input.len() {
                app.agent_input.remove(app.agent_input_cursor);
            }
            None
        }
        KeyCode::Left => {
            app.agent_input_cursor = app.agent_input_cursor.saturating_sub(1);
            None
        }
        KeyCode::Right => {
            if app.agent_input_cursor < app.agent_input.len() {
                app.agent_input_cursor += 1;
            }
            None
        }
        KeyCode::Home => {
            app.agent_input_cursor = 0;
            None
        }
        KeyCode::End => {
            app.agent_input_cursor = app.agent_input.len();
            None
        }
        _ => None,
    }
}

/// Strips control characters and trims whitespace from operator input
/// before it reaches the agent layer.
fn sanitize_input(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_control_characters() {
        assert_eq!(sanitize_input("hello\x00world"), "helloworld");
        assert_eq!(sanitize_input("buy\t100\n"), "buy100");
        assert_eq!(sanitize_input("\x1b[31mred\x1b[0m"), "[31mred[0m");
    }

    #[test]
    fn sanitize_trims_whitespace() {
        assert_eq!(sanitize_input("  buy 100  "), "buy 100");
    }

    #[test]
    fn sanitize_empty_and_whitespace_only() {
        assert_eq!(sanitize_input(""), "");
        assert_eq!(sanitize_input("   "), "");
        assert_eq!(sanitize_input("\n\t\r"), "");
    }

    #[test]
    fn sanitize_preserves_valid_unicode() {
        assert_eq!(sanitize_input("buy BTC/USD 0.5"), "buy BTC/USD 0.5");
        assert_eq!(sanitize_input("price ≥ 100"), "price ≥ 100");
    }

    #[test]
    fn max_input_length_rejects_at_limit() {
        let mut app = App::new();
        app.mode = Mode::Insert;
        app.focus = Focus::AgentInput;

        // Fill to exactly MAX_INPUT_LENGTH with ASCII chars
        app.agent_input = "a".repeat(MAX_INPUT_LENGTH);
        app.agent_input_cursor = MAX_INPUT_LENGTH;

        // One more char should be rejected
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        let action = handle_insert_mode(&mut app, key);
        assert!(action.is_none());
        assert_eq!(app.agent_input.len(), MAX_INPUT_LENGTH);
    }

    #[test]
    fn max_input_length_allows_under_limit() {
        let mut app = App::new();
        app.mode = Mode::Insert;
        app.focus = Focus::AgentInput;

        app.agent_input = "a".repeat(MAX_INPUT_LENGTH - 1);
        app.agent_input_cursor = MAX_INPUT_LENGTH - 1;

        let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
        let action = handle_insert_mode(&mut app, key);
        assert!(action.is_none());
        assert_eq!(app.agent_input.len(), MAX_INPUT_LENGTH);
        assert!(app.agent_input.ends_with('z'));
    }

    #[test]
    fn control_characters_rejected_during_input() {
        let mut app = App::new();
        app.mode = Mode::Insert;
        app.focus = Focus::AgentInput;

        // Try inserting a null byte
        let key = KeyEvent::new(KeyCode::Char('\0'), KeyModifiers::NONE);
        let action = handle_insert_mode(&mut app, key);
        assert!(action.is_none());
        assert!(app.agent_input.is_empty());
    }
}

/// Handles keys in confirm mode (dialogs).
fn handle_confirm_mode(app: &mut App, key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => {
            app.mode = Mode::Normal;
            Some(Action::ConfirmOrder)
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.pending_order = None;
            app.mode = Mode::Normal;
            None
        }
        _ => None,
    }
}

/// Handles keys in the risk parameters edit overlay.
fn handle_risk_edit_mode(app: &mut App, key: KeyEvent) -> Option<Action> {
    let state = app.risk_edit.as_mut()?;

    if state.editing {
        return handle_risk_field_edit(app, key);
    }

    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            if state.selected < RiskEditState::FIELD_COUNT - 1 {
                state.selected += 1;
            }
            None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.selected = state.selected.saturating_sub(1);
            None
        }

        // Toggle bool (intraday field, index 1)
        KeyCode::Char(' ') => {
            if state.selected == 1 {
                state.params.intraday = !state.params.intraday;
            }
            None
        }

        // Start editing a numeric field
        KeyCode::Enter | KeyCode::Char('i') => {
            if state.selected != 1 {
                // Pre-fill with current value
                state.input = state.field_value(state.selected);
                state.cursor = state.input.len();
                state.editing = true;
            }
            None
        }

        // Save and close
        KeyCode::Char('s') => {
            let params = state.params.clone();
            app.agent_risk_params = params.clone();
            app.risk_edit = None;
            app.mode = Mode::Normal;
            Some(Action::SaveRiskParams(params))
        }

        // Cancel and close
        KeyCode::Esc => {
            app.risk_edit = None;
            app.mode = Mode::Normal;
            None
        }

        _ => None,
    }
}

/// Handles keys when editing a numeric field in the risk overlay.
fn handle_risk_field_edit(app: &mut App, key: KeyEvent) -> Option<Action> {
    let state = app.risk_edit.as_mut().expect("editing requires risk_edit");

    match key.code {
        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
            state.input.insert(state.cursor, c);
            state.cursor += 1;
            None
        }
        KeyCode::Backspace => {
            if state.cursor > 0 {
                state.cursor -= 1;
                state.input.remove(state.cursor);
            }
            None
        }
        KeyCode::Left => {
            state.cursor = state.cursor.saturating_sub(1);
            None
        }
        KeyCode::Right => {
            if state.cursor < state.input.len() {
                state.cursor += 1;
            }
            None
        }
        KeyCode::Enter => {
            // Commit the edited value
            match state.selected {
                0 => {
                    if let Ok(v) = state.input.parse::<u32>() {
                        state.params.trades_per_month = v;
                    }
                }
                2 => {
                    if let Ok(v) = state.input.parse::<rust_decimal::Decimal>() {
                        state.params.trade_size_eur = v;
                    }
                }
                3 => {
                    if let Ok(v) = state.input.parse::<rust_decimal::Decimal>() {
                        state.params.stop_loss_eur = v;
                    }
                }
                _ => {}
            }
            state.editing = false;
            state.input.clear();
            state.cursor = 0;
            None
        }
        KeyCode::Esc => {
            // Cancel field edit (stay in overlay)
            state.editing = false;
            state.input.clear();
            state.cursor = 0;
            None
        }
        _ => None,
    }
}

/// Handles keys in the API keys edit overlay.
fn handle_api_keys_mode(app: &mut App, key: KeyEvent) -> Option<Action> {
    let state = app.api_keys_edit.as_mut()?;

    if state.editing {
        return handle_api_key_field_edit(app, key);
    }

    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            if state.selected < ApiKeysEditState::FIELD_COUNT - 1 {
                state.selected += 1;
            }
            None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.selected = state.selected.saturating_sub(1);
            None
        }

        // Start editing a field
        KeyCode::Enter | KeyCode::Char('i') => {
            state.input.clear();
            state.cursor = 0;
            state.editing = true;
            None
        }

        // Save and close
        KeyCode::Char('s') => {
            // Collect new values
            let values = std::array::from_fn(|i| state.fields[i].new_value.clone());

            // Check: if Kraken key is set but secret is not (or vice versa)
            let kraken_key_provided = values[1].is_some() || state.fields[1].was_set;
            let kraken_secret_provided = values[2].is_some() || state.fields[2].was_set;
            if kraken_key_provided != kraken_secret_provided {
                app.show_error("Kraken API key and secret must both be set");
                return None;
            }

            // Count unchanged keys that were already set
            let unchanged: usize = state
                .fields
                .iter()
                .filter(|f| f.was_set && f.new_value.is_none())
                .count();

            if unchanged > 0 && values.iter().all(|v| v.is_none()) {
                app.show_error(format!("{unchanged} key(s) unchanged (already set)"));
                app.api_keys_edit = None;
                app.mode = Mode::Normal;
                return None;
            }

            app.api_keys_edit = None;
            app.mode = Mode::Normal;
            Some(Action::SaveApiKeys { values })
        }

        // Cancel and close
        KeyCode::Esc => {
            app.api_keys_edit = None;
            app.mode = Mode::Normal;
            None
        }

        _ => None,
    }
}

/// Handles keys when editing a field in the API keys overlay.
fn handle_api_key_field_edit(app: &mut App, key: KeyEvent) -> Option<Action> {
    let state = app.api_keys_edit.as_mut().expect("editing requires api_keys_edit");

    match key.code {
        KeyCode::Char(c) if !c.is_control() => {
            state.input.insert(state.cursor, c);
            state.cursor += c.len_utf8();
            None
        }
        KeyCode::Backspace => {
            if state.cursor > 0 {
                state.cursor -= 1;
                state.input.remove(state.cursor);
            }
            None
        }
        KeyCode::Delete => {
            if state.cursor < state.input.len() {
                state.input.remove(state.cursor);
            }
            None
        }
        KeyCode::Left => {
            state.cursor = state.cursor.saturating_sub(1);
            None
        }
        KeyCode::Right => {
            if state.cursor < state.input.len() {
                state.cursor += 1;
            }
            None
        }
        KeyCode::Home => {
            state.cursor = 0;
            None
        }
        KeyCode::End => {
            state.cursor = state.input.len();
            None
        }
        KeyCode::Enter => {
            // Commit the edited value
            let value = std::mem::take(&mut state.input);
            if !value.is_empty() {
                state.fields[state.selected].new_value = Some(value);
            }
            state.editing = false;
            state.cursor = 0;
            None
        }
        KeyCode::Esc => {
            // Cancel field edit (stay in overlay)
            state.editing = false;
            state.input.clear();
            state.cursor = 0;
            None
        }
        _ => None,
    }
}
