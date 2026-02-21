# Session Context

**Session ID:** 70adf678-1485-4aba-b63e-5c21ac203943

**Commit Message:** Implement the following plan:

# Plan: Order Book Checksum Verification

## Prompt

Implement the following plan:

# Plan: Order Book Checksum Verification

## Context

The `verify_checksum()` and `calculate_checksum()` functions exist in `src/models/book.rs` but are never called. After every incremental book update, the local order book state could silently diverge from the exchange's actual book if a WebSocket message is lost or corrupted. Adding checksum verification catches this immediately and auto-recovers via re-subscription.

## Files to Modify

### 1. `src/tui/app.rs` — Add staleness tracking to `OrderBookState`

Add three fields to `OrderBookState` (currently derives `Default`, so defaults are fine):

```rust
pub struct OrderBookState {
    // ... existing fields ...
    /// Whether the book has a known checksum mismatch.
    pub is_stale: bool,                          // default: false
    /// Consecutive checksum failures since last good state.
    pub checksum_failures: u8,                   // default: 0
    /// When the last re-snapshot was requested (for cooldown).
    pub last_resync_request: Option<Instant>,     // default: None
}
```

Replace `#[derive(Default)]` with a manual `Default` impl (or keep derive — all three new fields have valid `Default` impls: `false`, `0`, `None`). Since `bool`, `u8`, and `Option<T>` all derive `Default` correctly, the existing `#[derive(Default)]` still works — just add the fields.

### 2. `src/tui/event.rs` — Verify checksum after every book update

**Add import** at the top:
```rust
use crate::models::book::calculate_checksum;
```

**Add `Action` variant:**
```rust
pub enum Action {
    // ... existing ...
    /// Re-subscribe to a pair's book channel after checksum mismatch.
    ResyncBook(String),
}
```

**In the `Message::Book` handler** (after updating state and setting `state.checksum`), add verification. The key logic:

- After a **snapshot**: reset `is_stale`, `checksum_failures`, `last_resync_request` (fresh data)
- After an **incremental update**: call `calculate_checksum(&state.asks, &state.bids)` and compare to `data.checksum`
- On **mismatch**: increment failures, log `tracing::warn!`, check cooldown (5s), return `Action::ResyncBook(symbol)` if within retry limit (3 attempts)
- On **match**: reset `checksum_failures` to 0, clear `is_stale`

The handler currently processes `response.data` in a `for` loop and returns `None` at the end. Since we need to potentially return an `Action` from inside the loop, collect the first resync action and return it after the loop completes. This avoids short-circuiting processing of a multi-symbol response (rare, but defensive).

### 3. `src/main.rs` — Handle `ResyncBook` action

Add a match arm alongside the existing `SubscribePair`/`UnsubscribePair`:

```rust
tui::event::Action::ResyncBook(symbol) => {
    tracing::info!(symbol = %symbol, "resyncing order book");
    let mut guard = writer.lock().await;
    if let Some(ref mut w) = *guard {
        let symbols = vec![symbol];
        // Unsubscribe triggers Kraken to drop the subscription
        let _ = unsubscribe(w, &Channel::Book, &symbols, None).await;
        // Re-subscribe triggers a fresh snapshot
        let _ = subscribe_book(w, &symbols, BookDepth::D25, None).await;
    }
}
```

No sleep between unsubscribe/resubscribe — Kraken processes them sequentially on the same connection, and the snapshot will arrive as a new `Message::Book` with `tpe == "snapshot"`.

Add the needed imports in `main.rs`: `use leeson::websocket::unsubscribe;` (the rest are already imported).

### 4. `src/tui/tabs/trading_pair.rs` — Show stale indicator

In `render_orderbook()` (line 123), adjust the title when the book is stale:

```rust
let is_stale = app.orderbooks.get(symbol).is_some_and(|ob| ob.is_stale);
let title = if is_stale { " Order Book [STALE] " } else { " Order Book " };
let border_style = if is_stale {
    Style::default().fg(Color::Yellow)
} else if is_focused {
    Style::default().fg(Color::Cyan)
} else {
    Style::default().fg(Color::DarkGray)
};
```

## Constants

Defined in `event.rs` at the top of the `Message::Book` handler or as module-level `const`:

| Name | Value | Purpose |
|---|---|---|
| `MAX_CHECKSUM_FAILURES` | `3` | Stop retrying after this many consecutive failures |
| `RESYNC_COOLDOWN` | `5s` | Minimum interval between re-snapshot requests |

## What NOT to change

- `src/models/book.rs` — `calculate_checksum` and `verify_checksum` stay as-is. We'll call `calculate_checksum` directly since we're verifying the accumulated local state, not a `BookData` struct.
- No new dependencies needed.

## Verification

1. `cargo check` — compiles
2. `cargo clippy` — no warnings
3. `cargo test` — existing tests pass (checksum calc tests already exist via the book model)
4. Manual: `cargo run`, subscribe to a pair, observe that book updates flow with no warnings in `leeson.log`
5. To test mismatch recovery: temporarily corrupt an update (e.g., flip a bid price) and verify the log shows a warn + the book recovers with a fresh snapshot


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.REDACTED.jsonl

## Summary

Here's a summary of what was implemented across 4 files:

**`src/tui/app.rs`** — Added three fields to `OrderBookState`:
- `is_stale: bool` — marks the book as having a checksum mismatch
- `checksum_failures: u8` — tracks consecutive failures for retry limiting
- `last_resync_request: Option<Instant>` — cooldown timer between resync attempts

**`src/tui/event.rs`** — Core verification logic:
- Added `Action::ResyncBook(String)` variant
- After every **incremental** update, calls `calculate_checksum()` and compares to the exchange's checksum
- On **mismatch**: sets `is_stale`, increments failure count, logs a warning, and returns `ResyncBook` if within retry limit (3) and cooldown (5s)
- On **match**: resets `is_stale` and failure count
- On **snapshot**: resets all staleness state (fresh data from exchange)

**`src/main.rs`** — Handles `ResyncBook` by unsubscribing then re-subscribing to the book channel, which triggers Kraken to send a fresh snapshot.

**`src/tui/tabs/trading_pair.rs`** — Shows a yellow `[STALE]` indicator on the order book border when checksum verification fails.

## Key Actions

- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/app.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/tabs/trading_pair.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/models/book.rs
- - **Grep**: pub async fn unsubscribe
- - **Read**: /Users/denny/Documents/repos/leeson/src/websocket/subscription.rs
- - **Grep**: pub use.*unsubscribe|pub use.*subscribe_book
- - **Grep**: pub use
- - **Read**: /Users/denny/Documents/repos/leeson/src/websocket/mod.rs
