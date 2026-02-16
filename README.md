# Leeson

## Leveraged Execution Engine for Speculative Operations Network

A hybrid Rust/Python crypto trading platform connecting to the Kraken V2 WebSocket API. Rust handles low-latency exchange connectivity and a real-time TUI, while Python agents (via PyO3) implement AI-driven trading strategies using Anthropic's Claude API.

## Architecture

```
Python Agents (Claude API)
        │ PyO3
        ▼
   Rust Core ──► TUI (Ratatui)
        │
        ▼
  Kraken V2 WebSocket
  (public + authenticated)
```

## Project Structure

```
src/
├── main.rs             # Binary entry point: event loop, TUI, WebSocket orchestration
├── lib.rs              # Library root
├── auth.rs             # REST API auth via HMAC-SHA512 for WebSocket tokens
├── config.rs           # Configuration loading (config.toml + env vars)
├── error.rs            # Unified error type
├── tls.rs              # TLS with pinned root certificate
├── python.rs           # PyO3 module: exposes models and order types to Python
├── models/             # Typed Kraken V2 API message structures
│   ├── ticker.rs       #   Price summaries (bid, ask, last, volume)
│   ├── book.rs         #   Order book snapshots and updates
│   ├── trade.rs        #   Public trade executions
│   ├── candle.rs       #   OHLC candlestick bars
│   ├── instrument.rs   #   Asset and trading pair metadata
│   ├── execution.rs    #   Order status changes and fills
│   ├── balance.rs      #   Wallet balance snapshots and updates
│   ├── orders.rs       #   Level-3 individual order data
│   ├── add_order.rs    #   Place orders (with triggers, conditionals, STP)
│   ├── edit_order.rs   #   Replace orders (loses queue priority)
│   ├── amend_order.rs  #   Modify orders (preserves queue priority)
│   ├── cancel_order.rs #   Cancel by ID or client reference
│   ├── batch_add.rs    #   Place 2-15 orders atomically
│   ├── batch_cancel.rs #   Cancel 2-50 orders in one request
│   ├── cancel_all.rs   #   Cancel all open orders
│   └── cancel_after.rs #   Dead man's switch (auto-cancel on timeout)
├── tui/                # Terminal UI (Ratatui + Crossterm)
│   ├── app.rs          #   Application state and data buffers
│   ├── event.rs        #   Keyboard, WebSocket, and tick event handling
│   ├── ui.rs           #   Frame rendering
│   ├── terminal.rs     #   Terminal setup and teardown
│   ├── tabs/           #   Tab panels (agent interaction, pair details)
│   ├── components/     #   Reusable widgets (status bar, tab bar)
│   └── input/          #   Text input handling
└── websocket/          # Async WebSocket client (Tokio + Tungstenite)
    ├── connection.rs   #   Lifecycle: connect, reconnect, token refresh
    ├── subscription.rs #   Channel subscribe/unsubscribe
    ├── trading.rs      #   Order RPC commands
    └── handler.rs      #   Incoming message dispatch
```

## Usage

```sh
cargo build               # Debug build
cargo build --release      # Release build
cargo run                  # Run the TUI
cargo test                 # Run tests
cargo clippy               # Lint
cargo fmt                  # Format
```

### Python Extension

```sh
pip install maturin
maturin develop --release  # Build and install Python module
```

```python
import leeson
order = leeson.AddOrder(
    order_type=leeson.OrderType.Limit,
    side=leeson.OrderSide.Buy,
    symbol="BTC/USD",
    order_qty=Decimal("0.001"),
    limit_price=Decimal("50000"),
)
```
