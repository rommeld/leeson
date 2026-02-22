# Session Context

**Session ID:** 8521a13d-94ae-45b6-9d9b-27861dd6962d

**Commit Message:** Implement the following plan:

# Plan: Expose Trading Data to Python Age

## Prompt

Implement the following plan:

# Plan: Expose Trading Data to Python Agents

## Context

Python agents currently operate blind. Despite all data model types being PyO3-exported, agents receive **none** of the trading or market data flowing through the system. The agent protocol only supports: user messages, risk limits text, a text-only order result (which is never even sent), token state, and shutdown. Agents cannot see their fills, order statuses, prices, or balances.

Since agents run as separate Python subprocesses (JSON-lines over stdin/stdout), data must be serialized to JSON — PyO3 types can't be used directly.

## Changes

### 1. Add `Serialize` to data model types

All relevant structs only derive `Deserialize`. Add `Serialize` so they can be serialized to JSON for the agent pipe.

**Files:**
- `src/models/execution.rs` — `ExecutionData`, `Fee`, `Triggers`, `Contingent`
- `src/models/ticker.rs` — `TickerData`
- `src/models/trade.rs` — `TradeData`
- `src/models/balance.rs` — `BalanceData`, `WalletBalance`

Each file: change `use serde::Deserialize` to `use serde::{Deserialize, Serialize}` and add `Serialize` to the `#[derive(...)]`.

### 2. Expand the agent protocol (`src/agent.rs`)

**Add new `AgentCommand` variants:**
- `ExecutionUpdate(Vec<ExecutionData>)` — order status changes, fills
- `TickerUpdate(TickerData)` — throttled price snapshots
- `TradeUpdate(Vec<TradeData>)` — market trades
- `BalanceUpdate(Vec<BalanceData>)` — balance changes
- `OrderResponse { success, order_id, cl_ord_id, order_userref, error }` — structured order placement result (replaces unused `OrderResult`)

**Add matching `TuiToAgent` variants** (same structure, serialized to JSON).

**Update `spawn_stdin_writer`** match arms to handle the new variants.

**Remove** the old `OrderResult` variant from `AgentCommand` and `TuiToAgent` (it is never sent anywhere in `main.rs`).

### 3. Forward data streams in main event loop (`src/main.rs`)

Follow the existing intercept pattern (used for `TokenState`, `AgentReady`). Add intercepts before `tui::event::update()`:

- **Execution updates** — forward all `Message::Execution` data to agents (low-frequency, highest priority)
- **Balance updates** — forward all `Message::Balance` data to agents (low-frequency)
- **Order responses** — intercept `Message::OrderPlaced` and forward structured `OrderResponse` to agents (includes `order_id`, `cl_ord_id`, `error`)
- **Ticker updates** — forward with **per-symbol time-based throttle** (max once per 5 seconds per symbol) to avoid flooding agents
- **Trade updates** — forward all `Message::Trade` data to agents (moderate frequency, discrete events)

All intercepts pass the message through to TUI unchanged.

### 4. Add `cl_ord_id` to agent order pipeline

Currently `AgentToTui::PlaceOrder` has no `cl_ord_id` field, so agents can't correlate order responses.

- Add optional `cl_ord_id` field to `AgentToTui::PlaceOrder` and `Message::AgentOrderRequest` in `src/tui/event.rs`
- Thread it through the `AddOrderBuilder` via the existing `.with_cl_ord_id()` method (already exists at `add_order.rs:459`)
- Add `place_order()` convenience method to the Python `Agent` class

### 5. Update Python agent base class (`agents/leeson_agent.py`)

**Add callback methods** (all default to no-op for backward compatibility):
- `on_execution(data: list[dict])` — fills, order status changes
- `on_ticker(data: dict)` — price snapshots
- `on_trade(data: list[dict])` — market trades
- `on_balance(data: list[dict])` — balance changes
- `on_order_response(success, order_id, cl_ord_id, order_userref, error)` — structured order result
- `on_risk_limits(description: str)` — risk config (currently unhandled)
- `on_token_state(state: str)` — auth state (currently unhandled)

**Update `run()` dispatch loop** to route new message types to the callbacks.

**Add `place_order()` outbound method** with `cl_ord_id` support.

## Files to modify

| File | Changes |
|------|---------|
| `src/models/execution.rs` | Add `Serialize` derive |
| `src/models/ticker.rs` | Add `Serialize` derive |
| `src/models/trade.rs` | Add `Serialize` derive |
| `src/models/balance.rs` | Add `Serialize` derive |
| `src/agent.rs` | New `AgentCommand`/`TuiToAgent` variants, remove old `OrderResult` |
| `src/main.rs` | Data forwarding intercepts with ticker throttling |
| `src/tui/event.rs` | Add `cl_ord_id` to `AgentOrderRequest` handling |
| `agents/leeson_agent.py` | New callbacks, `place_order()`, updated dispatch loop |

## What's NOT included (intentionally)

- **Order book forwarding** — large, high-frequency data; agents can infer from ticker bid/ask
- **Candle forwarding** — lower priority, agents get trades and can compute their own
- **Per-agent subscriptions** — all agents receive all streams; opt-in adds complexity without clear benefit yet

## Verification

1. `cargo check` / `cargo build` — confirm Serialize derives compile
2. `cargo test` — all existing tests pass
3. `cargo clippy` — no new warnings
4. Manual: run the TUI, spawn an agent, place an order, verify the agent receives `execution_update` and `order_response` messages on stdin
5. Write a test agent that logs all received message types to verify the full data flow


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.REDACTED.jsonl

## Summary

All clean. Here's a summary of what was implemented:

### Changes Made

**1. Model types — added `Serialize`** (4 files)
- `src/models/execution.rs` — `ExecutionData`, `Fee`, `Triggers`, `Contingent`
- `src/models/ticker.rs` — `TickerData`
- `src/models/trade.rs` — `TradeData`
- `src/models/balance.rs` — `BalanceData`, `WalletBalance`

**2. Agent protocol — `src/agent.rs`**
- Replaced `OrderResult` variant with structured `OrderResponse` (success, order_id, cl_ord_id, order_userref, error)
- Added `ExecutionUpdate`, `TickerUpdate`, `TradeUpdate`, `BalanceUpdate` variants to both `AgentCommand` and `TuiToAgent`
- Added `cl_ord_id` field to `AgentToTui::PlaceOrder`
- Derived `Clone` on `AgentCommand` for multi-agent forwarding
- Updated `spawn_stdin_writer` to handle all new variants

**3. Order pipeline — `src/tui/event.rs`**
- Added `cl_ord_id` field to `Message::AgentOrderRequest`
- Threaded `cl_ord_id` through to `AddOrderBuilder` via `.with_cl_ord_id()`

**4. Data forwarding — `src/main.rs`**
- Added intercepts that forward Execution, Balance, Trade, OrderPlaced, and Ticker data to all agents
- Ticker updates are throttled to max once per 5 seconds per symbol
- All messages pass through to the TUI unchanged

**5. Python agent — `agents/leeson_agent.py`**
- Added callbacks: `on_execution`, `on_ticker`, `on_trade`, `on_balance`, `on_order_response`, `on_risk_limits`, `on_token_state`
- Added `place_order()` outbound method with `cl_ord_id` support
- Updated `run()` dispatch loop to route all new message types

**Verification:** 150 tests pass, clippy clean, fmt clean.

## Key Actions

- - **Read**: /Users/denny/Documents/repos/leeson/src/models/execution.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/models/ticker.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/models/trade.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/models/balance.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/agent.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Read**: /Users/denny/Documents/repos/leeson/agents/leeson_agent.py
- - **Grep**: pub struct AddOrderResponse
- - **Grep**: with_cl_ord_id
