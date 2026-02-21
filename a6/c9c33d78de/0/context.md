# Session Context

**Session ID:** 92406551-3300-48d7-b2f0-ac96919e2e2c

**Commit Message:** Implement the following plan:

# Plan: Python Agent → TUI Output Bridge

## Prompt

Implement the following plan:

# Plan: Python Agent → TUI Output Bridge

## Context

The TUI has three agent output panels (`app.agent_outputs[0..3]`) and a text input field, but Python agents have no way to write to them. The `SendToAgent1` action currently just echoes a placeholder. We need an IPC bridge so Python agents (running as separate processes) can:

1. **Receive user messages** from the TUI input field (Agent 1)
2. **Write response lines** to their output panels
3. **Stream reasoning** line-by-line as Claude API tokens arrive

## Approach: Subprocess with JSON-Lines over stdin/stdout

Each Python agent runs as a child process. Rust spawns it with piped stdin/stdout/stderr. Communication uses newline-delimited JSON — one JSON object per line, each with a `"type"` field for routing.

**Why stdin/stdout pipes:**
- Zero new dependencies (tokio already has `tokio::process`, serde_json is already present)
- Process isolation — Python crash doesn't take down the TUI
- Each subprocess gets its own bidirectional pipe pair — no multiplexing needed
- Aligns with "across a process boundary" architecture described in CLAUDE.md

## Wire Protocol

**Python → Rust (stdout):**
```json
{"type":"output","agent":0,"line":"Analyzing BTC/USD..."}
{"type":"ready"}
{"type":"error","message":"API rate limit exceeded"}
```

**Rust → Python (stdin):**
```json
{"type":"user_message","content":"What is the BTC spread?"}
{"type":"shutdown"}
```

Non-JSON stdout lines are passed through as raw output (so `print()` works for debugging).

## Files to Create

### `src/agent.rs` — Agent process manager

New module with:

- **`AgentCommand`** enum — `UserMessage(String)`, `Shutdown`
- **`AgentHandle`** struct — holds `mpsc::UnboundedSender<AgentCommand>` and `Child`
- **`AgentToTui`** / **`TuiToAgent`** — serde types for the JSON-lines protocol
- **`spawn_agent(agent_index, script_path, tx) -> Result<AgentHandle>`** — spawns the subprocess and three tokio tasks:
  - **stdout reader**: parses JSON-lines, sends `Message::AgentOutput` / `AgentReady` / `AgentExited` to the main channel. Non-JSON lines forwarded as raw output.
  - **stderr reader**: forwards lines as `[stderr] ...` to the agent's panel (shows Python tracebacks).
  - **stdin writer**: reads from `AgentCommand` channel, serializes to JSON-lines, writes to child stdin.
- Uses `kill_on_drop(true)` for automatic cleanup.

### `agents/leeson_agent.py` — Python bridge library (stdlib only)

```python
class Agent:
    def __init__(self, agent_index: int): ...
    def output(self, line: str, panel: int | None = None): ...
    def error(self, message: str): ...
    def ready(self): ...
    def on_message(self, content: str): ...   # override in subclass
    def on_shutdown(self): ...                # override in subclass
    def run(self): ...                        # main stdin read loop
```

- `output()` writes `{"type":"output","agent":N,"line":"..."}` to stdout + flush
- `run()` reads stdin line-by-line, parses JSON, dispatches to `on_message` / `on_shutdown`
- Uses only stdlib (`json`, `sys`) — no dependencies

### `agents/agent1.py` — Example interactive agent (echo for testing)

Simple subclass that echoes user messages back, proving the round-trip works.

## Files to Modify

### `src/lib.rs`
- Add `pub mod agent;`

### `src/tui/event.rs`
- Add three `Message` variants:
  - `AgentOutput { agent_index: usize, line: String }`
  - `AgentReady(usize)`
  - `AgentExited { agent_index: usize, error: Option<String> }`
- Handle them in `update()`: call `app.add_agent_output()` / `app.show_error()`

### `src/main.rs`
- After creating channels, spawn agent processes via `agent::spawn_agent()`
- Store `AgentHandle`s in an array `[Option<AgentHandle>; 3]`
- In `SendToAgent1` arm: send `AgentCommand::UserMessage` to agent 0's handle
- Before terminal restore: send `AgentCommand::Shutdown` to all running agents

## Streaming Reasoning

Python agents stream Claude API responses by accumulating tokens into a line buffer and flushing on newlines:

```python
buffer = ""
with client.messages.stream(...) as stream:
    for text in stream.text_stream:
        buffer += text
        while "\n" in buffer:
            line, buffer = buffer.split("\n", 1)
            self.output(line)
    if buffer:
        self.output(buffer)
```

Each `self.output()` call writes JSON to stdout → Rust reads it immediately → sends `Message::AgentOutput` → next render cycle shows it. With 250ms tick interval, lines appear near-instantly.

## Implementation Order

1. Create `src/agent.rs` with types and `spawn_agent()`
2. Add `Message` variants and handlers in `event.rs`
3. Add `pub mod agent` to `lib.rs`
4. Wire up `main.rs` to spawn Agent 1 and forward `SendToAgent1`
5. Create `agents/leeson_agent.py` bridge library
6. Create `agents/agent1.py` echo agent for testing
7. Build, run, type in TUI, verify echo round-trip

## Verification

1. `cargo check` — compiles without errors
2. `cargo clippy` — no warnings
3. `cargo test` — existing tests still pass
4. Manual test: `cargo run`, switch to Agent tab, type a message, see it echoed back by Agent 1
5. Kill the Python process externally → verify TUI shows "Agent 1 exited" and doesn't crash


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.REDACTED.jsonl

## Summary

Here's a summary of everything implemented:

### New files
- **`src/agent.rs`** — Agent process manager with `AgentCommand`, `AgentHandle`, JSON-lines wire protocol types, and `spawn_agent()` that creates three tokio tasks (stdout reader, stderr reader, stdin writer)
- **`agents/leeson_agent.py`** — Python base class with `output()`, `error()`, `ready()`, `on_message()`, `on_shutdown()`, and `run()` loop (stdlib only)
- **`agents/agent1.py`** — Echo agent for testing the round-trip

### Modified files
- **`src/lib.rs`** — Added `pub mod agent`
- **`src/tui/event.rs`** — Added `AgentOutput`, `AgentReady`, `AgentExited` message variants and their handlers in `update()`
- **`src/main.rs`** — Spawns Agent 1 on startup, forwards `SendToAgent1` to the agent's command channel, sends `Shutdown` to all agents before terminal restore

### Verification
- `cargo check` — compiles cleanly
- `cargo clippy` — zero warnings
- `cargo test` — all 86 tests pass
- Python echo agent round-trip tested: stdin JSON → `ready` + `Echo: hello world` on stdout

## Key Actions

- - **Read**: /Users/denny/Documents/repos/leeson/src/lib.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Read**: /Users/denny/Documents/repos/leeson/Cargo.toml
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/app.rs
- - **Bash**: Check if agents directory exists
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/mod.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/error.rs
- - **TaskCreate**: Create the agent process manager module with AgentCommand, AgentHandle, wire protocol serde types, and spawn_agent() function that spawns a Python subprocess with stdin/stdout/stderr pipes and three tokio tasks for I/O.
- - **TaskCreate**: Add AgentOutput, AgentReady, AgentExited variants to the Message enum and handle them in the update() function.
