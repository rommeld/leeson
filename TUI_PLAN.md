# TUI Implementation Plan for Leeson

## Overview

Build a terminal user interface using Ratatui to interact with the Kraken WebSocket API. The TUI provides a tabbed interface with an Agent control tab and dynamic trading pair tabs.

## Scope

- **Symbols**: Multiple pairs (BTC/USD, ETH/USD, configurable list)
- **Authentication**: Full order management (create, cancel, amend orders)
- **Input**: Keyboard only (vim-style navigation, no mouse)
- **Tabs**: Agent tab + one tab per selected trading pair

## Architecture

**Pattern**: Elm Architecture (Model-Update-View) with async tokio integration

**Backend**: Crossterm (cross-platform, Ratatui default)

## Tab Structure

```
┌─────────────────────────────────────────────────────────────────────┐
│ [Agent] [BTC/USD] [ETH/USD] [SOL/USD]                              │
└─────────────────────────────────────────────────────────────────────┘
     │         │
     │         └── Trading Pair Tabs (dynamic, one per selected pair)
     │
     └── Agent Tab (always present)
```

## Layout: Agent Tab

```
┌─────────────────────────────────────────────────────────────────────┐
│ [Agent] [BTC/USD] [ETH/USD]                           Online │ 1/2 │
├─────────────────────────────────────────────────────────────────────┤
│                        AGENT OUTPUT                                 │
│ ┌─────────────────────┬─────────────────────┬─────────────────────┐ │
│ │ Agent 1             │ Agent 2             │ Agent 3             │ │
│ │─────────────────────│─────────────────────│─────────────────────│ │
│ │ Analyzing BTC/USD...│ Executing strategy  │ Risk check passed   │ │
│ │ Found pattern: bull │ Order placed: BUY   │ Position size OK    │ │
│ │ Confidence: 78%     │ Waiting for fill... │ Margin available    │ │
│ │ ...                 │ ...                 │ ...                 │ │
│ └─────────────────────┴─────────────────────┴─────────────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│                      ACCOUNT OVERVIEW                               │
│ ┌───────────────────────────────────────────────────────────────┐   │
│ │ Balance: $50,234.56    P&L Today: +$1,234.56 (+2.5%)          │   │
│ │ Equity:  $52,100.00    P&L Total: +$8,456.78 (+20.2%)         │   │
│ │ Margin:  $12,000.00    Open Positions: 3                      │   │
│ └───────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────┤
│                      EXECUTED TRADES (All Pairs)                    │
│ ┌───────────────────────────────────────────────────────────────┐   │
│ │ Time       Symbol    Side   Qty      Price      P&L           │   │
│ │ 12:34:56   BTC/USD   BUY    0.1000   97,234.00  -             │   │
│ │ 12:30:12   ETH/USD   SELL   2.5000   3,456.00   +$45.23       │   │
│ │ 12:25:00   BTC/USD   SELL   0.0500   97,100.00  +$123.45      │   │
│ └───────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────┤
│                      PAIR SELECTOR                                  │
│ ┌───────────────────────────────────────────────────────────────┐   │
│ │ [x] BTC/USD   [x] ETH/USD   [ ] SOL/USD   [ ] XRP/USD         │   │
│ │ [ ] DOGE/USD  [ ] ADA/USD   [ ] DOT/USD   [ ] LINK/USD        │   │
│ └───────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────┤
│ > Enter command for agents...                                       │
├─────────────────────────────────────────────────────────────────────┤
│ [Tab]switch tab [Space]toggle pair [Enter]send [?]help [q]quit     │
└─────────────────────────────────────────────────────────────────────┘
```

## Layout: Trading Pair Tab

```
┌─────────────────────────────────────────────────────────────────────┐
│ [Agent] [BTC/USD] [ETH/USD]                           Online │ 2/3 │
├─────────────────────────────────────────────────────────────────────┤
│ BTC/USD  ▲ 97,234.50  Bid: 97,230.00  Ask: 97,235.00  +2.3%        │
├──────────────────────────────────┬──────────────────────────────────┤
│         ORDER BOOK               │            CHART                 │
│ ┌──────────────────────────────┐ │ ┌──────────────────────────────┐ │
│ │ ASK                          │ │ │ [Candle] [Line]              │ │
│ │ 97,250.00    0.5000  ▒▒▒▒    │ │ │                              │ │
│ │ 97,245.00    1.2000  ▒▒▒▒▒▒  │ │ │     ▄▄                       │ │
│ │ 97,240.00    0.8500  ▒▒▒▒    │ │ │    ▄██▄    ▄▄                │ │
│ │ 97,235.00    0.3500  ▒▒      │ │ │   ▄████   ▄██▄  ▄▄           │ │
│ ├──────────────────────────────┤ │ │  ▄█████▄ ▄████ ▄██▄          │ │
│ │ BID                          │ │ │ ▄██████████████████▄         │ │
│ │ 97,230.00    2.1000  ▒▒▒▒▒▒▒ │ │ │ ████████████████████         │ │
│ │ 97,225.00    0.9500  ▒▒▒▒▒   │ │ │                              │ │
│ │ 97,220.00    1.8000  ▒▒▒▒▒▒  │ │ │ 1m  5m  15m  1h  4h  1d      │ │
│ │ 97,215.00    0.4500  ▒▒▒     │ │ └──────────────────────────────┘ │
│ └──────────────────────────────┘ │                                  │
├──────────────────────────────────┼──────────────────────────────────┤
│         TRADES                   │         ORDERS                   │
│ ┌──────────────────────────────┐ │ ┌──────────────────────────────┐ │
│ │ Time      Side   Qty   Price │ │ │ [Open] [Executed]            │ │
│ │ 12:34:56  BUY   0.01  97234  │ │ │                              │ │
│ │ 12:34:55  SELL  0.25  97233  │ │ │ ID        Side  Price   Qty  │ │
│ │ 12:34:54  BUY   0.05  97234  │ │ │ OA123...  BUY   95000  0.10  │ │
│ │ 12:34:53  SELL  0.10  97232  │ │ │ OB456...  SELL  99000  0.05  │ │
│ │ 12:34:52  BUY   0.02  97233  │ │ │                              │ │
│ └──────────────────────────────┘ │ └──────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│ [n]ew order [c]ancel [e]dit [Tab]switch tab [?]help [q]quit        │
└─────────────────────────────────────────────────────────────────────┘
```

## File Structure

```
src/
├── lib.rs                    # Add: pub mod tui;
├── main.rs                   # Modify: integrate TUI startup
├── tui/
│   ├── mod.rs               # Module exports
│   ├── app.rs               # Application state (Model)
│   ├── event.rs             # Message enum, event handling
│   ├── ui.rs                # Layout and rendering coordinator
│   ├── terminal.rs          # Terminal setup/teardown
│   ├── tabs/
│   │   ├── mod.rs
│   │   ├── agent.rs         # Agent tab layout and rendering
│   │   └── trading_pair.rs  # Trading pair tab layout and rendering
│   ├── components/
│   │   ├── mod.rs
│   │   ├── tab_bar.rs       # Tab navigation bar
│   │   ├── ticker.rs        # Price summary header
│   │   ├── orderbook.rs     # Order book depth widget
│   │   ├── trades.rs        # Trade tape widget
│   │   ├── chart.rs         # Chart widget (candle/line toggle)
│   │   ├── orders.rs        # Orders widget (open/executed toggle)
│   │   ├── status.rs        # Connection status bar
│   │   ├── agent_output.rs  # Agent output stream panels (x3)
│   │   ├── agent_input.rs   # Agent command input field
│   │   ├── account.rs       # Account balance/P&L display
│   │   ├── pair_selector.rs # Multi-select crypto pair picker
│   │   ├── executed_trades.rs # All executed trades table
│   │   └── order_form.rs    # Order entry modal
│   └── input/
│       ├── mod.rs
│       ├── form.rs          # Form field state
│       ├── text_input.rs    # Text input handling
│       └── keybindings.rs   # Key mappings
├── websocket/
│   └── handler.rs           # Add: process_messages_to_channel()
```

## Dependencies to Add

```toml
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }
unicode-width = "0.2"
```

## Key Bindings

### Global

| Key | Action |
|-----|--------|
| `q` | Quit application |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `?` | Help overlay |
| `Esc` | Cancel/close modal |

### Agent Tab

| Key | Action |
|-----|--------|
| `Enter` | Send command to agents |
| `Space` | Toggle selected pair in selector |
| `j/k` | Navigate pair selector |
| `1/2/3` | Focus agent output panel 1/2/3 |

### Trading Pair Tab

| Key | Action |
|-----|--------|
| `n` | New order |
| `c` | Cancel selected order |
| `e` | Edit selected order |
| `h/l` | Switch panel focus |
| `j/k` | Navigate within panel |
| `o` | Toggle Open/Executed orders |
| `g` | Toggle Candle/Line chart |
| `1-6` | Chart timeframe (1m/5m/15m/1h/4h/1d) |

## Application State

```rust
pub struct App {
    // -- Tab State --
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub selected_pairs: Vec<String>,

    // -- Agent State --
    pub agent_outputs: [VecDeque<String>; 3],
    pub agent_input: String,
    pub agent_input_cursor: usize,

    // -- Account State --
    pub balance: Decimal,
    pub equity: Decimal,
    pub margin_used: Decimal,
    pub pnl_today: Decimal,
    pub pnl_total: Decimal,
    pub executed_trades_all: VecDeque<ExecutedTrade>,

    // -- Per-Symbol Market Data --
    pub tickers: HashMap<String, TickerData>,
    pub orderbooks: HashMap<String, OrderBookState>,
    pub recent_trades: HashMap<String, VecDeque<TradeData>>,
    pub candles: HashMap<String, VecDeque<CandleData>>,

    // -- Per-Symbol Order State --
    pub open_orders: HashMap<String, Vec<OrderData>>,
    pub executed_orders: HashMap<String, VecDeque<ExecutionData>>,

    // -- UI State --
    pub focus: Focus,
    pub mode: Mode,
    pub chart_type: ChartType,          // Candle or Line
    pub chart_timeframe: Timeframe,     // 1m, 5m, 15m, 1h, 4h, 1d
    pub orders_view: OrdersView,        // Open or Executed
    pub pair_selector_index: usize,

    // -- Connection State --
    pub connection_status: ConnectionStatus,
    pub last_heartbeat: Option<Instant>,
    pub authenticated: bool,
}

pub enum Tab {
    Agent,
    TradingPair(String),  // Symbol name
}

pub enum ChartType {
    Candle,
    Line,
}

pub enum Timeframe {
    M1, M5, M15, H1, H4, D1,
}

pub enum OrdersView {
    Open,
    Executed,
}
```

## Event Loop Design

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Keyboard      │     │   WebSocket     │     │   Timer         │
│   Events        │     │   Messages      │     │   Ticks         │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 ▼
                        ┌─────────────────┐
                        │  mpsc::channel  │
                        │   Message enum  │
                        └────────┬────────┘
                                 ▼
                        ┌─────────────────┐
                        │    update()     │
                        │  (mutate App)   │
                        └────────┬────────┘
                                 ▼
                        ┌─────────────────┐
                        │     view()      │
                        │   (render UI)   │
                        └─────────────────┘
```

## Implementation Phases

### Phase 1: Foundation

1. Add dependencies to `Cargo.toml`
2. Create `src/tui/mod.rs` - module structure
3. Create `src/tui/terminal.rs` - setup/teardown
4. Create `src/tui/app.rs` - App struct with tab state
5. Create `src/tui/event.rs` - Message enum, event loop
6. Create `src/tui/ui.rs` - main render coordinator
7. Create `src/tui/components/tab_bar.rs` - tab navigation
8. Update `src/main.rs` - start TUI

**Milestone**: App starts with tab bar, Tab key switches tabs, `q` quits

### Phase 2: Agent Tab - Basic

1. Create `src/tui/tabs/agent.rs` - agent tab layout
2. Create `src/tui/components/agent_output.rs` - 3 output panels
3. Create `src/tui/components/agent_input.rs` - text input field
4. Create `src/tui/input/text_input.rs` - input handling
5. Create `src/tui/components/pair_selector.rs` - pair picker

**Milestone**: Agent tab displays, text input works, can select pairs

### Phase 3: Agent Tab - Account Info

1. Create `src/tui/components/account.rs` - balance/P&L display
2. Create `src/tui/components/executed_trades.rs` - trades table
3. Add `process_messages_to_channel()` to handler
4. Wire execution messages to account state

**Milestone**: Account info displays, executed trades populate

### Phase 4: Trading Pair Tab - Market Data

1. Create `src/tui/tabs/trading_pair.rs` - pair tab layout
2. Create `src/tui/components/ticker.rs` - price header
3. Create `src/tui/components/orderbook.rs` - order book
4. Create `src/tui/components/trades.rs` - trade tape
5. Dynamic tab creation when pairs selected

**Milestone**: Selecting a pair creates its tab with live data

### Phase 5: Trading Pair Tab - Charts

1. Create `src/tui/components/chart.rs` - chart widget
2. Implement candle chart rendering
3. Implement line chart rendering
4. Add timeframe switching (1m-1d)
5. Add chart type toggle (g key)

**Milestone**: Charts display with timeframe/type switching

### Phase 6: Trading Pair Tab - Orders

1. Create `src/tui/components/orders.rs` - orders widget
2. Implement open orders view
3. Implement executed orders view
4. Add view toggle (o key)
5. Create `src/tui/components/order_form.rs` - order modal
6. Wire to `add_order()`, `cancel_order()`

**Milestone**: Full order management per pair

### Phase 7: Polish

1. Help overlay (`?` key)
2. Confirmation dialogs
3. Error display in status bar
4. Graceful disconnect handling
5. Subscription management on pair toggle

**Milestone**: Production-ready TUI

## Critical Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` | Add ratatui, crossterm, unicode-width |
| `src/lib.rs` | Add `pub mod tui;` |
| `src/main.rs` | Replace logging with TUI startup |
| `src/websocket/handler.rs` | Add `process_messages_to_channel()` |
| `src/websocket/subscription.rs` | Add dynamic subscribe/unsubscribe for pair changes |

## Subscription Management

When pairs are toggled in the selector:
- **Adding pair**: Subscribe to Ticker, Book, Candles, Trades for that symbol
- **Removing pair**: Unsubscribe from those channels, remove tab
- **Tab state**: Tabs are dynamically created/removed based on selection

## Future: Agent Integration

The agent components are placeholders for future Python agent integration:
- **Agent Input**: Commands sent to Python agents via IPC
- **Agent Outputs**: Streaming responses from 3 concurrent agents
- **Integration point**: Will use process boundary communication (stdin/stdout, Unix socket, or similar)

## Verification

1. `cargo build` - compiles without errors
2. `cargo clippy` - no warnings
3. `cargo test` - existing tests pass
4. Manual testing:
   - Agent tab displays with all components
   - Text input accepts and displays text
   - Pair selector toggles pairs
   - Selecting pair creates new tab
   - Deselecting pair removes tab
   - Tab switching works (Tab/Shift+Tab)
   - Per-pair data displays correctly
   - Order book updates in real-time
   - Chart toggles between candle/line
   - Chart timeframe changes
   - Orders toggle between open/executed
   - Order creation/cancellation works
   - Account balance/P&L displays
   - `q` exits cleanly
