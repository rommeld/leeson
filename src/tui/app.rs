//! Application state for the TUI.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use rust_decimal::Decimal;

use crate::models::book::PriceLevel;
use crate::models::candle::CandleData;
use crate::models::execution::ExecutionData;
use crate::models::ticker::TickerData;
use crate::models::trade::TradeData;

/// Maximum number of items to keep in history buffers.
const MAX_HISTORY_SIZE: usize = 100;

/// Maximum number of agent output lines per panel.
const MAX_AGENT_OUTPUT_LINES: usize = 50;

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
    /// Time of last heartbeat received.
    pub last_heartbeat: Option<Instant>,
    /// Whether we have an authenticated session.
    pub authenticated: bool,

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
            agent_input: String::new(),
            agent_input_cursor: 0,

            balance: Decimal::ZERO,
            equity: Decimal::ZERO,
            margin_used: Decimal::ZERO,
            pnl_today: Decimal::ZERO,
            pnl_total: Decimal::ZERO,
            executed_trades_all: VecDeque::with_capacity(MAX_HISTORY_SIZE),

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

            connection_status: ConnectionStatus::Disconnected,
            last_heartbeat: None,
            authenticated: false,

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
            if output.len() >= MAX_AGENT_OUTPUT_LINES {
                output.pop_front();
            }
            output.push_back(line);
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

/// Error message with timestamp for auto-clear.
#[derive(Clone, Debug)]
pub struct ErrorDisplay {
    /// The error message.
    pub message: String,
    /// When the error was shown.
    pub timestamp: Instant,
}
