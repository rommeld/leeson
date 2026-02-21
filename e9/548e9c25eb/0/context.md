# Session Context

**Session ID:** 1e41a28f-e935-4194-b83b-b2c706cde192

**Commit Message:** Implement the following plan:

# Plan: Token Lifecycle & Agent Notificat

## Prompt

Implement the following plan:

# Plan: Token Lifecycle & Agent Notification

## Context

The WebSocket auth token (valid 15 min) is currently tracked only by creation time (`token_fetched_at`) with a 1-minute buffer before refresh (14 min). This is fragile — there's no visibility into token health, no notification to agents when the token is expiring, and no tracking of when the token was last *used* (vs. created). Agents that submit orders near token expiry risk silent failures.

This plan:
1. Increases the refresh buffer to 3 minutes (refresh at 12 min)
2. Tracks token *usage* time (when the token was last used for an authenticated operation)
3. Adds a token-state notification channel so agents know whether it's safe to submit orders

## Token State Machine

```
                    ┌─────────────────┐
  no credentials ──▶│   Unavailable   │
                    └─────────────────┘

  fetch success ──▶ ┌─────────────────┐  9 min elapsed   ┌─────────────────┐
                    │     Valid       │ ────────────────▶ │  ExpiringSoon   │
                    └─────────────────┘                   └─────────────────┘
                          ▲                                       │
                          │               12 min elapsed          ▼
                    ┌─────────────────┐◀──────────────── ┌─────────────────┐
                    │     Valid       │   (reconnect)    │   Refreshing    │
                    └─────────────────┘                  └─────────────────┘
```

## Files to Modify

### 1. `src/tui/app.rs` — Add `TokenState` enum and field

Add alongside existing state enums (`ConnectionStatus`, etc.):

```rust
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
    pub fn label(&self) -> &'static str {
        match self {
            TokenState::Unavailable => "unavailable",
            TokenState::Valid => "valid",
            TokenState::ExpiringSoon => "expiring_soon",
            TokenState::Refreshing => "refreshing",
        }
    }
}
```

Add field to `App`:
```rust
pub token_state: TokenState,
```

Initialize as `TokenState::Unavailable` in `App::new()`.

### 2. `src/websocket/connection.rs` — Timer changes + state emission

**Constants:**
```rust
/// Refresh token after 12 minutes (3-minute buffer before 15-min expiry).
const TOKEN_REFRESH_INTERVAL: Duration = Duration::from_secs(12 * 60);

/// Warn agents after 9 minutes that the token is aging.
const TOKEN_WARNING_THRESHOLD: Duration = Duration::from_secs(9 * 60);
```

**New field** on `ConnectionManager`:
```rust
pub struct ConnectionManager {
    // ... existing fields ...
    /// When the current token was last used for an authenticated operation.
    token_last_used: Option<Instant>,
}
```

**New `ConnectionCommand` variant:**
```rust
pub enum ConnectionCommand {
    // ... existing ...
    /// The token was used to submit an authenticated request (e.g., order placement).
    TokenUsed,
}
```

**`run()` changes:**
- After `fetch_token()` succeeds and private connection is set up, emit `Message::TokenState(TokenState::Valid)`.
- If no credentials, emit `Message::TokenState(TokenState::Unavailable)`.
- Before reconnect on `TokenExpired`, emit `Message::TokenState(TokenState::Refreshing)`.
- After connection error (entering backoff), emit `Message::TokenState(TokenState::Refreshing)`.

**`read_loop()` changes:**
- Add a second timer for `TOKEN_WARNING_THRESHOLD` (alongside existing refresh deadline).
- When warning timer fires, emit `Message::TokenState(TokenState::ExpiringSoon)` and continue the loop (don't exit).
- Handle `ConnectionCommand::TokenUsed` by updating `self.token_last_used = Some(Instant::now())` and logging it.
- Log token age and last-used time on token-related events for observability.

The `read_loop` `tokio::select!` gains one new branch:
```rust
() = &mut warning_sleep => {
    let _ = self.tx.send(Message::TokenState(TokenState::ExpiringSoon));
    info!(
        token_age_secs = token_fetched_at.elapsed().as_secs(),
        last_used = ?self.token_last_used.map(|t| t.elapsed()),
        "token approaching expiry"
    );
    // warning_sleep is now completed — won't fire again
}
```

### 3. `src/tui/event.rs` — New `Message::TokenState` variant

Add to the `Message` enum:
```rust
/// Token lifecycle state change.
TokenState(crate::tui::app::TokenState),
```

Handle in `update()`:
```rust
Message::TokenState(state) => {
    app.token_state = state;
    None
}
```

### 4. `src/agent.rs` — Forward token state to agents

**New `AgentCommand` variant:**
```rust
pub enum AgentCommand {
    // ... existing ...
    /// Token state changed — agents should know if orders can be submitted.
    TokenState(String),
}
```

**New `TuiToAgent` variant:**
```rust
enum TuiToAgent {
    // ... existing ...
    TokenState { state: String },
}
```

**`spawn_stdin_writer`** — handle new variant:
```rust
AgentCommand::TokenState(state) => TuiToAgent::TokenState { state },
```

### 5. `src/main.rs` — Route token state to agents

In the main loop, intercept `Message::TokenState` before passing to `update()`:

```rust
Message::TokenState(state) => {
    // Forward to all running agents
    for handle in agents.iter().flatten() {
        let _ = handle.commands.send(AgentCommand::TokenState(state.label().to_string()));
    }
    // Also pass through to update() for app.token_state
    tui::event::update(&mut app, Message::TokenState(state));
    continue;
}
```

When an order is successfully submitted (`Action::SubmitOrder` → `Approved` or `Action::ConfirmOrder`), send usage notification:
```rust
let _ = cmd_tx.send(ConnectionCommand::TokenUsed);
```

### 6. `src/websocket/mod.rs` — Re-export `ConnectionCommand`

Ensure `ConnectionCommand` is accessible from `main.rs` (already re-exported based on current imports).

## Implementation Order

1. Add `TokenState` enum to `src/tui/app.rs` + field on `App` — compiles immediately
2. Add `Message::TokenState` variant to `src/tui/event.rs` + handler in `update()`
3. Add `AgentCommand::TokenState` + `TuiToAgent::TokenState` to `src/agent.rs`
4. Modify `src/websocket/connection.rs` — timer constants, warning timer, state emission, `TokenUsed` command, `token_last_used` field
5. Wire in `src/main.rs` — intercept `Message::TokenState`, send to agents, send `TokenUsed` on order submission

## What NOT to Change

- `src/auth.rs` — token fetching logic unchanged
- `src/risk/` — risk guard is orthogonal to token state
- `src/models/` — no model changes needed

## Verification

1. `cargo check` — compiles
2. `cargo clippy` — no warnings
3. `cargo test` — existing tests pass (no new tests needed for timer-based state machine)
4. Manual: `cargo run` with credentials → observe log lines:
   - `"Fetched authentication token"` + `TokenState::Valid` sent
   - After ~9 min: `"token approaching expiry"` log + `TokenState::ExpiringSoon` sent
   - After ~12 min: reconnect cycle + `TokenState::Refreshing` then `TokenState::Valid`
5. Manual: `cargo run` without credentials → `TokenState::Unavailable` from start
6. Submit an order → confirm `TokenUsed` is logged in connection manager


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.REDACTED.jsonl

## Summary

All done. Here's a summary of the changes:

**`src/tui/app.rs`** — Added `TokenState` enum (`Unavailable`, `Valid`, `ExpiringSoon`, `Refreshing`) with a `label()` method for agent serialization, and a `token_state` field on `App`.

**`src/tui/event.rs`** — Added `Message::TokenState` variant and its handler in `update()` to set `app.token_state`.

**`src/agent.rs`** — Added `AgentCommand::TokenState(String)` and `TuiToAgent::TokenState { state }` so agents receive token lifecycle notifications over stdin.

**`src/websocket/connection.rs`** — 
- Increased refresh buffer from 1 min to 3 min (`TOKEN_REFRESH_INTERVAL` = 12 min)
- Added `TOKEN_WARNING_THRESHOLD` at 9 min
- Added `token_last_used: Option<Instant>` field
- Added `ConnectionCommand::TokenUsed` variant
- Emits `TokenState::Valid`/`Unavailable` after token fetch, `Refreshing` before reconnect, `ExpiringSoon` when warning timer fires
- Handles `TokenUsed` command to track last usage time
- Logs token age and last-used time on token events

**`src/main.rs`** — Intercepts `Message::TokenState` to forward to all running agents, sends `ConnectionCommand::TokenUsed` after successful order submission (both direct and confirmed).

## Key Actions

- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/app.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/websocket/connection.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/agent.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/websocket/mod.rs
- - **TaskCreate**: Add TokenState enum and field on App struct
- - **TaskCreate**: Add Message::TokenState variant and handler in update()
- - **TaskCreate**: Add AgentCommand::TokenState + TuiToAgent::TokenState variant and handle in spawn_stdin_writer
- - **TaskCreate**: Timer constants, warning timer, state emission, TokenUsed command, token_last_used field
