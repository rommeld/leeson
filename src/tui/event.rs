//! Event handling for the TUI.

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use crate::models::balance::BalanceResponse;
use crate::models::book::BookUpdateResponse;
use crate::models::candle::CandleUpdateResponse;
use crate::models::execution::ExecutionUpdateResponse;
use crate::models::ticker::TickerUpdateResponse;
use crate::models::trade::TradeUpdateResponse;
use crate::models::{
    AddOrderResponse, AmendOrderResponse, CancelAllResponse, CancelOrderResponse,
    StatusUpdateResponse,
};

use super::app::{App, AssetBalance, Focus, MAX_ORDERBOOK_HISTORY, Mode, OrderBookSnapshot, Tab};

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
    AgentOutput {
        agent_index: usize,
        line: String,
    },
    /// Agent subprocess signaled readiness.
    AgentReady(usize),
    /// Agent subprocess exited.
    AgentExited {
        agent_index: usize,
        error: Option<String>,
    },

    /// Request to quit the application.
    Quit,
}

/// Spawns a task that polls for terminal events and sends them to a channel.
pub fn spawn_event_reader(tx: mpsc::UnboundedSender<Message>) {
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
                    if tx.send(Message::Input(Event::Key(key))).is_err() {
                        break;
                    }
                }
                Ok(Some(CrosstermEvent::Resize(w, h))) => {
                    if tx.send(Message::Input(Event::Resize(w, h))).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
}

/// Spawns a task that sends periodic tick events.
pub fn spawn_tick_timer(tx: mpsc::UnboundedSender<Message>, interval_ms: u64) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
        loop {
            interval.tick().await;
            if tx.send(Message::Input(Event::Tick)).is_err() {
                break;
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
            let is_snapshot = response.tpe == "snapshot";

            for data in response.data {
                let state = app.orderbooks.entry(data.symbol.clone()).or_default();

                if is_snapshot {
                    // Snapshot: replace entire book
                    state.bids = data.bids;
                    state.asks = data.asks;
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
                            state.bids.push(level);
                            // Keep bids sorted highest to lowest
                            state.bids.sort_by(|a, b| b.price.cmp(&a.price));
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
                            state.asks.push(level);
                            // Keep asks sorted lowest to highest
                            state.asks.sort_by(|a, b| a.price.cmp(&b.price));
                        }
                    }
                }

                state.checksum = data.checksum;
                state.last_update = Some(std::time::Instant::now());

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
            None
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
            None
        }
        Message::Reconnecting => {
            app.connection_status = super::app::ConnectionStatus::Reconnecting;
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
        Message::AgentExited { agent_index, error } => {
            let msg = match error {
                Some(e) => format!("[agent exited: {e}]"),
                None => "[agent exited]".to_string(),
            };
            app.add_agent_output(agent_index, msg);
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
    /// Place a new order.
    PlaceOrder,
    /// Cancel an order.
    CancelOrder(String),
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
            let command = std::mem::take(&mut app.agent_input);
            app.agent_input_cursor = 0;
            app.mode = Mode::Normal;
            if !command.is_empty() {
                return Some(Action::SendToAgent1(command));
            }
            None
        }
        KeyCode::Char(c) => {
            app.agent_input.insert(app.agent_input_cursor, c);
            app.agent_input_cursor += 1;
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

/// Handles keys in confirm mode (dialogs).
fn handle_confirm_mode(app: &mut App, key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => {
            app.mode = Mode::Normal;
            // TODO: Execute confirmed action
            None
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.mode = Mode::Normal;
            None
        }
        _ => None,
    }
}
