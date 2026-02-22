# Session Context

**Session ID:** e269eb60-0609-498b-b3e0-6d9c81e5e109

**Commit Message:** Implement the following plan:

# Plan: Stream Agent Output and Show Reas

## Prompt

Implement the following plan:

# Plan: Stream Agent Output and Show Reasoning in TUI

## Context

The multi-agent trading system currently uses batch `agent.run()` calls — the LLM generates its entire response before anything appears in the TUI. This means operators wait seconds with no feedback while agents "think." Switching to streaming output shows the LLM's reasoning in real-time as it generates, giving operators immediate visibility into agent decision-making.

## Approach

Switch from pydantic-ai's `agent.run()` to the streaming `agent.iter()` API. As the LLM generates text, send incremental deltas over the existing JSON-lines bridge to the Rust TUI. The TUI accumulates deltas in a per-panel buffer, flushing complete lines to the output panel immediately and showing the in-progress partial line with a cursor indicator.

## Changes

### 1. Python: Bridge streaming functions

**File:** `agents/multi_agent/bridge.py`

Add two new functions:
- `send_stream_delta(panel, delta)` — sends `{"type": "stream_delta", "agent": N, "delta": "text"}`
- `send_stream_end(panel)` — sends `{"type": "stream_end", "agent": N}`

### 2. Python: Reusable streaming helper

**File:** `agents/multi_agent/models.py`

Add `run_agent_streamed()` that encapsulates the common streaming pattern:

```python
async def run_agent_streamed(agent, prompt, *, deps, history, model, panel) -> list:
    async with agent.iter(prompt, deps=deps, message_history=history, model=model) as agent_run:
        async for node in agent_run:
            if isinstance(node, ModelRequestNode):
                async with node.stream(agent_run.ctx) as stream:
                    async for chunk in stream.stream_text(delta=True):
                        send_stream_delta(panel, chunk)
                send_stream_end(panel)
    record_usage(deps, agent_run.result)
    return agent_run.all_messages()[-30:]
```

Each `ModelRequestNode` gets its own stream/flush cycle, so tool calls (which happen between model nodes) appear cleanly between streamed text.

### 3. Python: Convert all agent `run_*` functions

Replace the `agent.run()` + `record_usage()` + `output_to_panel(result.output)` pattern with `run_agent_streamed()` in:

| File | Functions |
|------|-----------|
| `agents/user_agent.py` | `run_once()` |
| `agents/market_agent.py` | `run_on_user_request()`, `run_on_ticker()`, `run_on_consultation()` |
| `agents/risk_agent.py` | `run_on_trade_idea()`, `run_on_market_analysis()`, `run_on_execution_update()`, `run_position_review()` |
| `agents/execution_agent.py` | `run_on_approved_order()`, `run_on_close_position()` |
| `agents/ideation_agent.py` | `run_periodic()` |

Tool functions (e.g., `send_trade_idea`, `place_order`) are unchanged — they still call `output_to_panel()` directly for immediate output.

### 4. Rust: New `AgentToTui` variants and stdout parsing

**File:** `src/agent.rs`

Add to the `AgentToTui` enum:
```rust
StreamDelta { agent: usize, delta: String },
StreamEnd { agent: usize },
```

Handle in `spawn_stdout_reader()` by sending `Message::AgentStreamDelta` / `Message::AgentStreamEnd`.

### 5. Rust: New `Message` variants and event handling

**File:** `src/tui/event.rs`

Add to `Message` enum:
```rust
AgentStreamDelta { agent_index: usize, delta: String },
AgentStreamEnd { agent_index: usize },
```

Handle in `update()`:
- `AgentStreamDelta` → `app.append_stream_delta(agent_index, &delta)`
- `AgentStreamEnd` → `app.flush_stream_buffer(agent_index)`

### 6. Rust: Streaming buffer state management

**File:** `src/tui/app.rs`

Add field: `pub agent_stream_buffers: [String; 3]`

Add methods:
- **`append_stream_delta(index, delta)`**: Appends delta to buffer. Scans for `\n` — complete lines get flushed to `agent_outputs` via existing `add_agent_output()`. Partial remainder stays in buffer.
- **`flush_stream_buffer(index)`**: Flushes any remaining buffer content as a final line. Called on `stream_end`.

### 7. Rust: Render streaming line with cursor indicator

**File:** `src/tui/tabs/agent.rs`

In `render_agent_outputs()`:
- Count `effective_total = agent_outputs.len() + (1 if buffer non-empty)`
- Use `effective_total` for scroll indicator and scrollbar
- When pinned to bottom and buffer is non-empty, append a `ListItem` with the buffer text + yellow "▍" cursor

When the operator scrolls up (unpins), the partial line is hidden so they can read history undisturbed.

## Verification

1. `cargo check` — Rust compiles with new variants and state
2. `cargo test` — Unit tests for `append_stream_delta` / `flush_stream_buffer` buffer logic
3. `cargo clippy` — No lint warnings
4. Manual test: run the TUI, type a message to the User Agent, observe text streaming character-by-character in panel 0 with yellow cursor indicator
5. Verify tool calls still work: trigger a market analysis request, confirm tool output (`[idea]`, `[risk]` prefixed lines) appears between streamed text


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.REDACTED.jsonl

## Summary

All tasks complete. Here's a summary of the changes:

## Summary

### Python changes (4 files)

1. **`agents/multi_agent/bridge.py`** — Added `send_stream_delta(panel, delta)` and `send_stream_end(panel)` functions that send `stream_delta` / `stream_end` JSON-lines messages to the TUI.

2. **`agents/multi_agent/models.py`** — Added `run_agent_streamed()` helper that uses pydantic-ai's `agent.iter()` → `ModelRequestNode.stream()` → `stream_text(delta=True)` pipeline, sending each chunk via `send_stream_delta` and signaling `send_stream_end` after each model request node completes.

3. **`agents/multi_agent/agents/{user,market,risk,execution,ideation}_agent.py`** — Replaced every `agent.run()` + `record_usage()` + `output_to_panel()` pattern with a single `run_agent_streamed()` call. Tool functions (e.g., `send_trade_idea`, `place_order`) remain unchanged — they still emit immediate `output_to_panel` lines.

### Rust changes (4 files)

4. **`src/agent.rs`** — Added `StreamDelta { agent, delta }` and `StreamEnd { agent }` variants to `AgentToTui`, with routing in `spawn_stdout_reader()` to the new `Message` variants.

5. **`src/tui/event.rs`** — Added `AgentStreamDelta` and `AgentStreamEnd` to the `Message` enum, with handling in `update()` that calls `app.append_stream_delta()` / `app.flush_stream_buffer()`.

6. **`src/tui/app.rs`** — Added `agent_stream_buffers: [String; 3]` field and two methods:
   - `append_stream_delta()` — accumulates text, flushes complete `\n`-terminated lines to `agent_outputs`
   - `flush_stream_buffer()` — emits any remaining partial content as a final line
   
   Plus 6 unit tests covering: complete line flushing, partial line buffering, multi-delta accumulation, flush on stream end, noop flush when empty, and invalid index bounds.

7. **`src/tui/tabs/agent.rs`** — Updated `render_agent_outputs()` to show an in-progress streaming line with a yellow "▍" cursor indicator when pinned to bottom. The partial line is hidden when the operator scrolls up.

### Verification
- `cargo check` — compiles clean
- `cargo test` — 129 tests pass (6 new + 123 existing)
- `cargo clippy` — no warnings
- `cargo fmt` — formatted

## Key Actions

- - **TaskCreate**: Add send_stream_delta() and send_stream_end() to agents/multi_agent/bridge.py
- - **TaskCreate**: Add reusable streaming helper function that encapsulates the agent.iter() streaming pattern
- - **TaskCreate**: Replace agent.run() + record_usage() + output_to_panel() pattern with run_agent_streamed() in all agent files
- - **TaskCreate**: Add StreamDelta/StreamEnd to AgentToTui enum and handle in spawn_stdout_reader()
- - **TaskCreate**: Add AgentStreamDelta/AgentStreamEnd to Message enum and handle in update()
- - **TaskCreate**: Add agent_stream_buffers field and append_stream_delta/flush_stream_buffer methods
- - **TaskCreate**: Update render_agent_outputs() to show partial streaming line with yellow cursor indicator
- - **TaskCreate**: Run cargo check, cargo test, and cargo clippy to verify everything compiles and passes
- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/bridge.py
- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/models.py
