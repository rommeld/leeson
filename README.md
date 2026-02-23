# Leeson

## Leveraged Execution Engine for Speculative Operations Network

A hybrid Rust/Python crypto trading platform connecting to the Kraken V2 WebSocket API. Rust handles low-latency exchange connectivity and a real-time TUI, while Python agents implement AI-driven trading strategies. Agents communicate with the Rust core over JSON-lines via stdin/stdout pipes.

## Architecture

```
Python Agents (Claude API)
        │ JSON-lines (stdin/stdout)
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

## Prerequisites

- **Rust** (stable, edition 2024 — requires Rust 1.85+)
- **Python 3.12+** and [uv](https://docs.astral.sh/uv/) (for running agents)
- **Supported platforms:** macOS (ARM64), Linux (x86\_64), Windows (x86\_64)

## Configuration

### Environment Variables

| Variable | Required | Default | Description |
| --- | --- | --- | --- |
| `KRAKEN_API_KEY` | No | — | Kraken API key for authenticated trading |
| `KRAKEN_API_SECRET` | No | — | Kraken API secret (must be set with `KRAKEN_API_KEY`) |
| `KRAKEN_WEBSOCKET_URL` | No | `wss://ws.kraken.com/v2` | Override the WebSocket endpoint |
| `LEESON_SIMULATION` | No | `false` | Set to `true` or `1` for simulation mode |
| `LEESON_TOKEN_INPUT_COST` | No | — | USD cost per 1M input tokens (for TUI cost display) |
| `LEESON_TOKEN_OUTPUT_COST` | No | — | USD cost per 1M output tokens (for TUI cost display) |
| `FIREWORKS_API_KEY` | For agents | — | Fireworks AI API key used by the Python agent |

Credentials can also be entered at runtime via the TUI (`a` key) or stored in the macOS Keychain. On macOS, stored keychain credentials are automatically loaded into the environment at startup.

### risk.json

A `risk.json` file must exist in the working directory. It defines order-level risk limits and confirmation thresholds.

```json
{
  "defaults": {
    "max_order_qty": "1.0",
    "max_notional_value": "100000",
    "confirm_above_notional": "50000",
    "max_trades_per_day": 50,
    "max_trades_per_week": 200,
    "max_trades_per_month": 500
  },
  "symbols": {
    "BTC/USD": {
      "max_order_qty": "0.5",
      "max_notional_value": "50000",
      "confirm_above_notional": "25000"
    }
  }
}
```

- `defaults` — Global limits applied to all trading pairs
- `symbols` — Optional per-symbol overrides; omitted fields inherit from `defaults`
- `confirm_above_notional` — Orders exceeding this value require operator confirmation in the TUI

### agent\_risk.json

An optional `agent_risk.json` file provides advisory parameters sent to agents. These are editable live via the TUI risk overlay (`r` key) and saved back on `s`.

```json
{
  "trades_per_month": 10,
  "intraday": false,
  "trade_size_eur": "100",
  "stop_loss_eur": "20",
  "cost_per_1m_input_tokens": "0",
  "cost_per_1m_output_tokens": "0"
}
```

## Building and Running

```sh
cargo build                        # Debug build
cargo build --release              # Release build
cargo run                          # Run the TUI
LEESON_SIMULATION=true cargo run   # Run in simulation mode (no real orders)
```

Agents are spawned from the TUI. The Rust core launches `uv run --directory agents python -m multi_agent` as a child process and communicates via JSON-lines over stdin/stdout.

## TUI Key Bindings

### Global

| Key | Action |
| --- | --- |
| `q` | Quit (Normal mode) |
| `Esc` | Return to Normal mode |
| `Tab` / `Shift+Tab` | Next / previous tab |
| `r` | Open risk parameters overlay |
| `a` | Open API keys overlay |

### Agent Tab (Normal Mode)

| Key | Action |
| --- | --- |
| `1` `2` `3` | Focus agent output panel |
| `j` / `k` | Scroll down / up in focused panel |
| `g` / `G` | Jump to top / bottom |
| `h` / `l` | Navigate focus left / right |
| `i` or `Enter` | Enter Insert mode (agent input) |
| `Space` | Toggle selected trading pair |

### Agent Tab (Insert Mode)

| Key | Action |
| --- | --- |
| `Enter` | Submit command to agent |
| Typing / `Backspace` | Edit input text |
| `Left` / `Right` / `Home` / `End` | Move cursor |

### Trading Pair Tab

| Key | Action |
| --- | --- |
| `h` / `l` / `j` / `k` | Navigate focus between panels |
| `1`–`6` | Chart timeframe (1m, 5m, 15m, 1h, 4h, 1d) |
| `g` | Toggle chart type |
| `o` | Toggle orders view (open / executed) |

### Confirm Overlay

| Key | Action |
| --- | --- |
| `y` or `Enter` | Confirm pending order |
| `n` or `Esc` | Cancel pending order |

## Python Extension

The `python` feature exposes Rust types to Python via PyO3. This is separate from agent communication (which uses JSON-lines).

```sh
pip install maturin
maturin develop --release --features python,extension-module
```

```python
from decimal import Decimal
import leeson

order = leeson.AddOrder(
    order_type=leeson.OrderType.Limit,
    side=leeson.OrderSide.Buy,
    symbol="BTC/USD",
    order_qty=Decimal("0.001"),
    limit_price=Decimal("50000"),
)
```

Feature flags:

- `python` — Enables the PyO3 dependency
- `extension-module` — Required for maturin builds (kept separate so `cargo test --features python` can link against libpython)

## Development

```sh
cargo test                 # Run all tests
cargo clippy               # Lint
cargo fmt                  # Format
cargo fmt --check          # Verify formatting without modifying files
cargo check                # Fast compile check
```
