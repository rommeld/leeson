//! Application state for the TUI.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use rust_decimal::Decimal;

use crate::models::add_order::AddOrderParams;
use crate::models::book::PriceLevel;
use crate::models::candle::CandleData;
use crate::models::execution::ExecutionData;
use crate::models::ticker::TickerData;
use crate::models::trade::TradeData;
use crate::risk::config::AgentRiskParams;

/// Maximum number of items to keep in history buffers.
const MAX_HISTORY_SIZE: usize = 100;

/// Maximum number of agent output lines per panel.
const MAX_AGENT_OUTPUT_LINES: usize = 50;

/// Maximum number of order book snapshots to retain in history.
pub const MAX_ORDERBOOK_HISTORY: usize = 20;

/// Maximum number of order book levels retained per side (bids/asks).
pub const MAX_BOOK_DEPTH: usize = 1000;

/// Central application state container.
pub struct App {
    // -- Tab State --
    /// List of available tabs.
    pub tabs: Vec<Tab>,
    /// Index of the currently active tab.
    pub active_tab: usize,
    /// Symbols selected for trading (shown as tabs).
    pub selected_pairs: Vec<String>,
    /// All available trading pairs.
    pub available_pairs: Vec<String>,

    // -- Agent State --
    /// Output streams for three agent panels.
    pub agent_outputs: [VecDeque<String>; 3],
    /// Scroll state for each agent output panel.
    pub agent_scroll: [ScrollState; 3],
    /// Partial-line buffers for streaming agent output.
    pub agent_stream_buffers: [String; 3],
    /// Current text in the agent input field.
    pub agent_input: String,
    /// Cursor position in the agent input field.
    pub agent_input_cursor: usize,

    // -- Account State --
    /// Account balance in USD.
    pub balance: Decimal,
    /// Account equity (balance + unrealized P&L).
    pub equity: Decimal,
    /// Margin currently in use.
    pub margin_used: Decimal,
    /// Profit/loss for today.
    pub pnl_today: Decimal,
    /// Total profit/loss.
    pub pnl_total: Decimal,
    /// All executed trades across all pairs.
    pub executed_trades_all: VecDeque<ExecutedTrade>,
    /// Per-asset balances from the balances channel.
    pub asset_balances: HashMap<String, AssetBalance>,

    // -- Per-Symbol Market Data --
    /// Latest ticker data per symbol.
    pub tickers: HashMap<String, TickerData>,
    /// Order book state per symbol.
    pub orderbooks: HashMap<String, OrderBookState>,
    /// Recent trades per symbol.
    pub recent_trades: HashMap<String, VecDeque<TradeData>>,
    /// Candle data per symbol.
    pub candles: HashMap<String, VecDeque<CandleData>>,

    // -- Per-Symbol Order State --
    /// Open orders per symbol.
    pub open_orders: HashMap<String, Vec<ExecutionData>>,
    /// Executed orders per symbol.
    pub executed_orders: HashMap<String, VecDeque<ExecutionData>>,

    // -- UI State --
    /// Current focus within the active tab.
    pub focus: Focus,
    /// Current input mode.
    pub mode: Mode,
    /// Chart type (candle or line).
    pub chart_type: ChartType,
    /// Chart timeframe.
    pub chart_timeframe: Timeframe,
    /// Orders view (open or executed).
    pub orders_view: OrdersView,
    /// Index in the pair selector.
    pub pair_selector_index: usize,
    /// Error message to display (clears after timeout).
    pub error_message: Option<ErrorDisplay>,

    // -- Connection State --
    /// WebSocket connection status.
    pub connection_status: ConnectionStatus,
    /// Authentication token lifecycle state.
    pub token_state: TokenState,
    /// Time of last heartbeat received.
    pub last_heartbeat: Option<Instant>,
    /// Whether we have an authenticated session (credentials valid at startup).
    pub authenticated: bool,
    /// Whether the private WebSocket is currently connected.
    pub private_connected: bool,

    // -- Risk State --
    /// Order pending operator confirmation.
    pub pending_order: Option<PendingOrder>,
    /// Advisory risk parameters communicated to agents.
    pub agent_risk_params: AgentRiskParams,
    /// State for the risk parameters edit overlay.
    pub risk_edit: Option<RiskEditState>,
    /// State for the API keys edit overlay.
    pub api_keys_edit: Option<ApiKeysEditState>,

    // -- Token Usage --
    /// Cumulative token usage from agent LLM calls.
    pub token_usage: TokenUsageStats,

    // -- Simulation --
    /// Whether the application is running in simulation mode.
    pub simulation: bool,
    /// Snapshot of simulation statistics for display.
    pub sim_stats: SimulationStats,

    // -- Internal --
    /// Flag to signal application should quit.
    pub should_quit: bool,
}

impl App {
    /// Creates a new App instance with default state.
    pub fn new() -> Self {
        Self {
            tabs: vec![Tab::Agent],
            active_tab: 0,
            selected_pairs: Vec::new(),
            available_pairs: vec![
                "BTC/USD".to_string(),
                "ETH/USD".to_string(),
                "SOL/USD".to_string(),
                "XRP/USD".to_string(),
                "DOGE/USD".to_string(),
                "ADA/USD".to_string(),
                "DOT/USD".to_string(),
                "LINK/USD".to_string(),
            ],

            agent_outputs: [
                VecDeque::with_capacity(MAX_AGENT_OUTPUT_LINES),
                VecDeque::with_capacity(MAX_AGENT_OUTPUT_LINES),
                VecDeque::with_capacity(MAX_AGENT_OUTPUT_LINES),
            ],
            agent_scroll: [ScrollState::default(); 3],
            agent_stream_buffers: Default::default(),
            agent_input: String::new(),
            agent_input_cursor: 0,

            balance: Decimal::ZERO,
            equity: Decimal::ZERO,
            margin_used: Decimal::ZERO,
            pnl_today: Decimal::ZERO,
            pnl_total: Decimal::ZERO,
            executed_trades_all: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            asset_balances: HashMap::new(),

            tickers: HashMap::new(),
            orderbooks: HashMap::new(),
            recent_trades: HashMap::new(),
            candles: HashMap::new(),

            open_orders: HashMap::new(),
            executed_orders: HashMap::new(),

            focus: Focus::AgentInput,
            mode: Mode::Normal,
            chart_type: ChartType::Candle,
            chart_timeframe: Timeframe::M1,
            orders_view: OrdersView::Open,
            pair_selector_index: 0,
            error_message: None,

            pending_order: None,
            agent_risk_params: AgentRiskParams::default(),
            risk_edit: None,
            api_keys_edit: None,

            connection_status: ConnectionStatus::Disconnected,
            token_state: TokenState::Unavailable,
            last_heartbeat: None,
            authenticated: false,
            private_connected: false,

            token_usage: TokenUsageStats::default(),

            simulation: false,
            sim_stats: SimulationStats::default(),

            should_quit: false,
        }
    }

    /// Returns the currently active tab.
    pub fn current_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    /// Switches to the next tab.
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
            self.update_focus_for_tab();
        }
    }

    /// Switches to the previous tab.
    pub fn previous_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = self
                .active_tab
                .checked_sub(1)
                .unwrap_or(self.tabs.len() - 1);
            self.update_focus_for_tab();
        }
    }

    /// Updates focus when switching tabs.
    fn update_focus_for_tab(&mut self) {
        match self.current_tab() {
            Tab::Agent => self.focus = Focus::AgentInput,
            Tab::TradingPair(_) => self.focus = Focus::OrderBook,
        }
    }

    /// Toggles selection of a trading pair.
    pub fn toggle_pair(&mut self, symbol: &str) {
        if let Some(pos) = self.selected_pairs.iter().position(|s| s == symbol) {
            // Remove pair and its tab
            self.selected_pairs.remove(pos);
            if let Some(tab_pos) = self
                .tabs
                .iter()
                .position(|t| matches!(t, Tab::TradingPair(s) if s == symbol))
            {
                self.tabs.remove(tab_pos);
                // Adjust active tab if needed
                if self.active_tab >= self.tabs.len() && self.active_tab > 0 {
                    self.active_tab = self.tabs.len() - 1;
                }
            }
        } else {
            // Add pair and its tab
            self.selected_pairs.push(symbol.to_string());
            self.tabs.push(Tab::TradingPair(symbol.to_string()));
        }
    }

    /// Checks if a pair is currently selected.
    pub fn is_pair_selected(&self, symbol: &str) -> bool {
        self.selected_pairs.iter().any(|s| s == symbol)
    }

    /// Adds a line to an agent output panel.
    pub fn add_agent_output(&mut self, agent_index: usize, line: String) {
        if agent_index < 3 {
            let output = &mut self.agent_outputs[agent_index];
            let was_at_max = output.len() >= MAX_AGENT_OUTPUT_LINES;
            if was_at_max {
                output.pop_front();
                // Approximate offset adjustment when not pinned
                let scroll = &mut self.agent_scroll[agent_index];
                if !scroll.pinned {
                    scroll.offset = scroll.offset.saturating_sub(1);
                }
            }
            output.push_back(line);
            // When pinned, offset is computed at render time from max_scroll
        }
    }

    /// Appends a streaming text delta to an agent's stream buffer.
    ///
    /// Complete lines (terminated by `\n`) are flushed immediately to
    /// `agent_outputs` via [`add_agent_output`]. Any partial remainder
    /// stays in the buffer for display as an in-progress line.
    pub fn append_stream_delta(&mut self, agent_index: usize, delta: &str) {
        if agent_index >= 3 {
            return;
        }
        self.agent_stream_buffers[agent_index].push_str(delta);

        // Flush complete lines
        while let Some(newline_pos) = self.agent_stream_buffers[agent_index].find('\n') {
            let line: String = self.agent_stream_buffers[agent_index]
                .drain(..=newline_pos)
                .collect();
            // Trim the trailing newline
            let line = line.trim_end_matches('\n').to_string();
            self.add_agent_output(agent_index, line);
        }
    }

    /// Flushes any remaining content in a streaming buffer as a final line.
    ///
    /// Called when the agent signals the end of a streaming response.
    pub fn flush_stream_buffer(&mut self, agent_index: usize) {
        if agent_index >= 3 {
            return;
        }
        if !self.agent_stream_buffers[agent_index].is_empty() {
            let line = std::mem::take(&mut self.agent_stream_buffers[agent_index]);
            self.add_agent_output(agent_index, line);
        }
    }

    /// Scrolls an agent output panel up by one line.
    pub fn scroll_agent_up(&mut self, agent_index: usize) {
        if agent_index < 3 {
            let scroll = &mut self.agent_scroll[agent_index];
            if scroll.offset > 0 {
                scroll.offset -= 1;
                scroll.pinned = false;
            }
        }
    }

    /// Scrolls an agent output panel down by one visual row.
    pub fn scroll_agent_down(&mut self, agent_index: usize) {
        if agent_index < 3 {
            let scroll = &mut self.agent_scroll[agent_index];
            if scroll.offset < scroll.max_scroll {
                scroll.offset += 1;
            }
            // Re-pin when scrolled to the bottom
            if scroll.offset >= scroll.max_scroll {
                scroll.pinned = true;
            }
        }
    }

    /// Scrolls an agent output panel to the top.
    pub fn scroll_agent_top(&mut self, agent_index: usize) {
        if agent_index < 3 {
            let scroll = &mut self.agent_scroll[agent_index];
            scroll.offset = 0;
            scroll.pinned = false;
        }
    }

    /// Scrolls an agent output panel to the bottom and re-pins.
    pub fn scroll_agent_bottom(&mut self, agent_index: usize) {
        if agent_index < 3 {
            let scroll = &mut self.agent_scroll[agent_index];
            scroll.offset = scroll.max_scroll;
            scroll.pinned = true;
        }
    }

    /// Sets an error message to display.
    pub fn show_error(&mut self, message: impl Into<String>) {
        self.error_message = Some(ErrorDisplay {
            message: message.into(),
            timestamp: Instant::now(),
        });
    }

    /// Clears error messages older than 5 seconds.
    pub fn clear_stale_errors(&mut self) {
        if let Some(ref error) = self.error_message
            && error.timestamp.elapsed() > std::time::Duration::from_secs(5)
        {
            self.error_message = None;
        }
    }

    /// Updates ticker data for a symbol.
    pub fn update_ticker(&mut self, symbol: String, data: TickerData) {
        self.tickers.insert(symbol, data);
    }

    /// Adds a trade to the recent trades for a symbol.
    pub fn add_trade(&mut self, symbol: &str, trade: TradeData) {
        let trades = self
            .recent_trades
            .entry(symbol.to_string())
            .or_insert_with(|| VecDeque::with_capacity(MAX_HISTORY_SIZE));
        if trades.len() >= MAX_HISTORY_SIZE {
            trades.pop_front();
        }
        trades.push_back(trade);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Tab types in the application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tab {
    /// Agent control and monitoring tab.
    Agent,
    /// Trading pair tab with market data and orders.
    TradingPair(String),
}

impl Tab {
    /// Returns the display title for the tab.
    pub fn title(&self) -> &str {
        match self {
            Tab::Agent => "Agent",
            Tab::TradingPair(symbol) => symbol,
        }
    }
}

/// Chart display type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChartType {
    #[default]
    Candle,
    Line,
}

impl ChartType {
    /// Toggles between chart types.
    pub fn toggle(&mut self) {
        *self = match self {
            ChartType::Candle => ChartType::Line,
            ChartType::Line => ChartType::Candle,
        };
    }
}

/// Chart timeframe options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Timeframe {
    #[default]
    M1,
    M5,
    M15,
    H1,
    H4,
    D1,
}

impl Timeframe {
    /// Returns the display label.
    pub fn label(&self) -> &'static str {
        match self {
            Timeframe::M1 => "1m",
            Timeframe::M5 => "5m",
            Timeframe::M15 => "15m",
            Timeframe::H1 => "1h",
            Timeframe::H4 => "4h",
            Timeframe::D1 => "1d",
        }
    }

    /// Returns the interval value for the Kraken API.
    pub fn interval(&self) -> i32 {
        match self {
            Timeframe::M1 => 1,
            Timeframe::M5 => 5,
            Timeframe::M15 => 15,
            Timeframe::H1 => 60,
            Timeframe::H4 => 240,
            Timeframe::D1 => 1440,
        }
    }
}

/// Orders view mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OrdersView {
    #[default]
    Open,
    Executed,
}

impl OrdersView {
    /// Toggles between views.
    pub fn toggle(&mut self) {
        *self = match self {
            OrdersView::Open => OrdersView::Executed,
            OrdersView::Executed => OrdersView::Open,
        };
    }
}

/// UI focus targets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Focus {
    // Agent tab
    #[default]
    AgentInput,
    AgentOutput1,
    AgentOutput2,
    AgentOutput3,
    PairSelector,
    OpenOrdersAll,
    ExecutedTradesAll,

    // Trading pair tab
    OrderBook,
    Trades,
    Chart,
    Orders,
}

/// Input mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Confirm,
    RiskEdit,
    ApiKeys,
}

/// Authentication token lifecycle state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TokenState {
    /// No credentials configured.
    #[default]
    Unavailable,
    /// Token is fresh and valid.
    Valid,
    /// Token is approaching expiry (past 9-minute mark).
    ExpiringSoon,
    /// Token is being refreshed (reconnecting).
    Refreshing,
}

impl TokenState {
    /// Returns a stable label for serialization to agents.
    pub fn label(&self) -> &'static str {
        match self {
            TokenState::Unavailable => "unavailable",
            TokenState::Valid => "valid",
            TokenState::ExpiringSoon => "expiring_soon",
            TokenState::Refreshing => "refreshing",
        }
    }
}

/// WebSocket connection status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

impl ConnectionStatus {
    /// Returns a display string for the status.
    pub fn label(&self) -> &'static str {
        match self {
            ConnectionStatus::Disconnected => "Offline",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Connected => "Online",
            ConnectionStatus::Reconnecting => "Reconnecting...",
        }
    }
}

/// Order book state for a symbol.
#[derive(Clone, Debug, Default)]
pub struct OrderBookState {
    /// Bid levels (price, quantity).
    pub bids: Vec<PriceLevel>,
    /// Ask levels (price, quantity).
    pub asks: Vec<PriceLevel>,
    /// Checksum for validation.
    pub checksum: u32,
    /// Last update time.
    pub last_update: Option<Instant>,
    /// Historical snapshots of best bid/ask.
    pub history: VecDeque<OrderBookSnapshot>,
    /// Whether the book has a known checksum mismatch.
    pub is_stale: bool,
    /// Consecutive checksum failures since last good state.
    pub checksum_failures: u8,
    /// When the last re-snapshot was requested (for cooldown).
    pub last_resync_request: Option<Instant>,
}

/// A historical snapshot of order book state.
#[derive(Clone, Debug)]
pub struct OrderBookSnapshot {
    /// Timestamp from the exchange (RFC3339 format).
    pub timestamp: String,
    /// Best bid price at the time.
    pub best_bid: Decimal,
    /// Best ask price at the time.
    pub best_ask: Decimal,
    /// Spread (best_ask - best_bid).
    pub spread: Decimal,
}

/// An executed trade for display in the all-trades table.
#[derive(Clone, Debug)]
pub struct ExecutedTrade {
    /// Timestamp of the trade.
    pub timestamp: String,
    /// Trading pair symbol.
    pub symbol: String,
    /// Trade side (buy/sell).
    pub side: String,
    /// Quantity traded.
    pub qty: Decimal,
    /// Execution price.
    pub price: Decimal,
    /// Realized P&L from this trade.
    pub pnl: Option<Decimal>,
}

/// Order pending operator confirmation before submission.
#[derive(Debug, Clone)]
pub struct PendingOrder {
    /// The order parameters to submit if confirmed.
    pub params: AddOrderParams,
    /// Human-readable reason why confirmation is required.
    pub reason: String,
}

/// State for the risk parameters edit overlay.
#[derive(Clone, Debug)]
pub struct RiskEditState {
    /// Index of the currently selected field (0..4).
    pub selected: usize,
    /// Whether the selected numeric field is being edited.
    pub editing: bool,
    /// Text buffer for the field being edited.
    pub input: String,
    /// Cursor position within the input buffer.
    pub cursor: usize,
    /// Working copy of parameters (committed on save).
    pub params: AgentRiskParams,
}

impl RiskEditState {
    /// Number of editable fields.
    pub const FIELD_COUNT: usize = 4;

    /// Creates a new edit state from existing parameters.
    pub fn new(params: &AgentRiskParams) -> Self {
        Self {
            selected: 0,
            editing: false,
            input: String::new(),
            cursor: 0,
            params: params.clone(),
        }
    }

    /// Returns the label for the field at the given index.
    pub fn field_label(index: usize) -> &'static str {
        match index {
            0 => "Trades/month",
            1 => "Intraday",
            2 => "Trade size (EUR)",
            3 => "Stop-loss (EUR)",
            _ => "",
        }
    }

    /// Returns the display value for the field at the given index.
    pub fn field_value(&self, index: usize) -> String {
        match index {
            0 => self.params.trades_per_month.to_string(),
            1 => if self.params.intraday { "Yes" } else { "No" }.to_string(),
            2 => self.params.trade_size_eur.to_string(),
            3 => self.params.stop_loss_eur.to_string(),
            _ => String::new(),
        }
    }
}

/// State for the API keys edit overlay.
#[derive(Clone, Debug)]
pub struct ApiKeysEditState {
    /// Index of the currently selected field (0..3).
    pub selected: usize,
    /// Whether the selected field is being edited.
    pub editing: bool,
    /// Text buffer for the field being edited.
    pub input: String,
    /// Cursor position within the input buffer.
    pub cursor: usize,
    /// Per-field state.
    pub fields: [ApiKeyField; 4],
}

impl Default for ApiKeysEditState {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiKeysEditState {
    /// Number of credential fields.
    pub const FIELD_COUNT: usize = 4;

    /// Creates a new edit state, checking which keys are already set.
    pub fn new() -> Self {
        use crate::credentials::{CredentialKey, is_set};

        let fields = CredentialKey::ALL.map(|key| ApiKeyField {
            was_set: is_set(key),
            new_value: None,
        });

        Self {
            selected: 0,
            editing: false,
            input: String::new(),
            cursor: 0,
            fields,
        }
    }

    /// Returns the display label for the field at the given index.
    pub fn field_label(index: usize) -> &'static str {
        use crate::credentials::CredentialKey;
        CredentialKey::ALL.get(index).map_or("", |k| k.label())
    }

    /// Returns a status string for the field at the given index.
    pub fn field_status(&self, index: usize) -> FieldStatus {
        let field = &self.fields[index];
        if field.new_value.is_some() {
            FieldStatus::NewValue
        } else if field.was_set {
            FieldStatus::Set
        } else {
            FieldStatus::NotSet
        }
    }
}

/// Display status of a credential field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldStatus {
    /// No value in keychain.
    NotSet,
    /// Value exists in keychain.
    Set,
    /// A new value has been entered.
    NewValue,
}

/// Per-field state in the API keys overlay.
#[derive(Clone, Debug)]
pub struct ApiKeyField {
    /// Whether this key was already set when the overlay was opened.
    pub was_set: bool,
    /// New value entered by the user (None = not edited).
    pub new_value: Option<String>,
}

/// Error message with timestamp for auto-clear.
#[derive(Clone, Debug)]
pub struct ErrorDisplay {
    /// The error message.
    pub message: String,
    /// When the error was shown.
    pub timestamp: Instant,
}

/// Cumulative token usage from agent LLM calls.
#[derive(Clone, Debug, Default)]
pub struct TokenUsageStats {
    /// Total input (prompt) tokens across all calls.
    pub input_tokens: u64,
    /// Total output (completion) tokens across all calls.
    pub output_tokens: u64,
    /// USD cost per million input tokens (from env var).
    pub input_cost_per_million: Option<Decimal>,
    /// USD cost per million output tokens (from env var).
    pub output_cost_per_million: Option<Decimal>,
}

impl TokenUsageStats {
    /// Total tokens (input + output).
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Estimated cost in USD, if rates are configured.
    pub fn estimated_cost(&self) -> Option<Decimal> {
        let million = Decimal::from(1_000_000);
        let input_cost = self
            .input_cost_per_million
            .map(|rate| rate * Decimal::from(self.input_tokens) / million);
        let output_cost = self
            .output_cost_per_million
            .map(|rate| rate * Decimal::from(self.output_tokens) / million);
        match (input_cost, output_cost) {
            (Some(i), Some(o)) => Some(i + o),
            (Some(i), None) => Some(i),
            (None, Some(o)) => Some(o),
            (None, None) => None,
        }
    }
}

/// Simulation performance statistics for TUI display.
#[derive(Clone, Debug, Default)]
pub struct SimulationStats {
    /// Cumulative realized P&L (after fees).
    pub realized_pnl: Decimal,
    /// Unrealized P&L from open positions at current market prices.
    pub unrealized_pnl: Decimal,
    /// Number of simulated trades executed this session.
    pub trade_count: usize,
    /// Net position per symbol.
    pub positions: HashMap<String, Decimal>,
    /// Weighted average entry price per symbol.
    pub avg_entry_prices: HashMap<String, Decimal>,
    /// Session duration in seconds.
    pub session_secs: u64,
}

/// Scroll state for a text output panel.
#[derive(Clone, Copy, Debug)]
pub struct ScrollState {
    /// Number of visual (wrapped) rows scrolled from the top of content.
    pub offset: usize,
    /// Whether the view is pinned to the bottom (auto-scrolls on new content).
    pub pinned: bool,
    /// Maximum valid scroll offset (updated each render frame).
    pub max_scroll: usize,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            offset: 0,
            pinned: true,
            max_scroll: 0,
        }
    }
}

/// Balance for a single asset.
#[derive(Clone, Debug)]
pub struct AssetBalance {
    /// Asset symbol (e.g., "BTC", "USD").
    pub asset: String,
    /// Total balance across all wallets.
    pub total: Decimal,
    /// Balance in spot wallet.
    pub spot: Decimal,
    /// Balance in earn wallet.
    pub earn: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_delta_flushes_complete_lines() {
        let mut app = App::new();
        app.append_stream_delta(0, "hello\nworld\n");
        assert_eq!(app.agent_outputs[0].len(), 2);
        assert_eq!(app.agent_outputs[0][0], "hello");
        assert_eq!(app.agent_outputs[0][1], "world");
        assert!(app.agent_stream_buffers[0].is_empty());
    }

    #[test]
    fn stream_delta_keeps_partial_line_in_buffer() {
        let mut app = App::new();
        app.append_stream_delta(0, "partial");
        assert_eq!(app.agent_outputs[0].len(), 0);
        assert_eq!(app.agent_stream_buffers[0], "partial");
    }

    #[test]
    fn stream_delta_accumulates_then_flushes() {
        let mut app = App::new();
        app.append_stream_delta(1, "hel");
        app.append_stream_delta(1, "lo\nwor");
        assert_eq!(app.agent_outputs[1].len(), 1);
        assert_eq!(app.agent_outputs[1][0], "hello");
        assert_eq!(app.agent_stream_buffers[1], "wor");

        app.append_stream_delta(1, "ld");
        assert_eq!(app.agent_outputs[1].len(), 1);
        assert_eq!(app.agent_stream_buffers[1], "world");
    }

    #[test]
    fn flush_stream_buffer_emits_remaining_content() {
        let mut app = App::new();
        app.append_stream_delta(2, "final partial");
        app.flush_stream_buffer(2);
        assert_eq!(app.agent_outputs[2].len(), 1);
        assert_eq!(app.agent_outputs[2][0], "final partial");
        assert!(app.agent_stream_buffers[2].is_empty());
    }

    #[test]
    fn flush_stream_buffer_noop_when_empty() {
        let mut app = App::new();
        app.flush_stream_buffer(0);
        assert_eq!(app.agent_outputs[0].len(), 0);
    }

    #[test]
    fn stream_delta_ignores_invalid_agent_index() {
        let mut app = App::new();
        app.append_stream_delta(5, "ignored");
        for i in 0..3 {
            assert!(app.agent_stream_buffers[i].is_empty());
            assert!(app.agent_outputs[i].is_empty());
        }
    }
}
