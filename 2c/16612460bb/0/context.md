# Session Context

**Session ID:** 1e41a28f-e935-4194-b83b-b2c706cde192

**Commit Message:** Use bounded_channel(capacity) with a documented capacity. Drop or log st

## Prompt

Use bounded_channel(capacity) with a documented capacity. Drop or log stale messages when the buffer is full. check @src/main.rs

## Summary

All clean. Here's a summary of the changes:

**`src/main.rs`** — Replaced both `unbounded_channel` calls with `channel`:
- Message channel: capacity **512** (absorbs WebSocket data bursts)
- ConnectionCommand channel: capacity **32** (infrequent subscribe/unsubscribe/token-used)
- All `cmd_tx.send()` → `cmd_tx.try_send()` with `warn!` on full

**`src/websocket/connection.rs`** — Changed `tx` from `UnboundedSender` to `Sender`, `cmd_rx` from `UnboundedReceiver` to `Receiver`. Added `try_send()` helper method that warns on `Full` and logs on `Closed`. In `read_loop()`, `Full` drops the message with a warning while `Closed` returns `Shutdown`.

**`src/tui/event.rs`** — Changed both spawner signatures to `Sender<Message>`. Key/resize events warn on drop; tick events drop silently (periodic, harmless to lose).

**`src/agent.rs`** — Changed `spawn_agent` and internal reader/writer signatures to `Sender<Message>`. All sends use `try_send()`.

## Key Actions

- - **Read**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Grep**: UnboundedSender|UnboundedReceiver|unbounded_channel
- - **Edit**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/websocket/connection.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/websocket/connection.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/websocket/connection.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/websocket/connection.rs
