# Session Context

**Session ID:** d3716988-e97a-4b46-85a9-82f65b9a1fc3

**Commit Message:** If the authenticated WebSocket connection fails but credentials are conf

## Prompt

If the authenticated WebSocket connection fails but credentials are configured, the app silently falls back to public-only mode. The operator has no indication that authenticated channels are unavailable. Implement indication for authenticated connection.

## Summary

All 100 tests pass, plus all integration and doc tests. Here's a summary of the changes:

**Problem**: When the authenticated WebSocket connection failed but credentials were configured, the status bar still showed "Auth" in green. The operator had no indication that executions/balances channels were unavailable.

**Changes across 4 files:**

- **`app.rs`** — Added `private_connected: bool` field to `App`, initialized to `false`.

- **`event.rs`** — Added `PrivateChannelStatus(bool)` message variant. The handler sets `app.private_connected`. Also resets `private_connected = false` on `Disconnected` and `Reconnecting` messages.

- **`connection.rs`** — Sends `PrivateChannelStatus` at every state transition:
  - `true` when private WS connects and subscribes successfully
  - `false` when token fetch fails (with credentials configured), private connection fails, private ping fails, or private stream errors/ends mid-session

- **`status_bar.rs`** — Three-way auth display:
  - **No Auth** (dark gray) — no credentials configured
  - **Auth** (green) — private WebSocket connected
  - **Auth Down** (yellow) — credentials valid but private WebSocket unavailable

## Key Actions

- - **Task**: Explore auth connection flow
- - **Read**: /Users/denny/Documents/repos/leeson/src/websocket/connection.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/components/status_bar.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/tui/app.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/tui/app.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
