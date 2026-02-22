# Session Context

**Session ID:** a2535cdf-302d-4854-a0d9-a8f766461674

**Commit Message:** Implement the following plan:

# Plan: Token Counter in TUI

## Context

## Prompt

Implement the following plan:

# Plan: Token Counter in TUI

## Context

The Python agents make LLM calls via pydantic-ai (Fireworks/MiniMax M2.5) but no token usage is tracked. The operator has no visibility into how many tokens agents burn or what it costs. This adds cumulative token tracking across all agent calls, surfaces the totals in the TUI status bar, and optionally shows estimated cost when the operator provides rates via environment variables.

## Data Flow

```
pydantic-ai RunResult.usage() → SharedState counters → bridge.send_token_usage()
  → JSON-lines stdout → Rust AgentToTui::TokenUsage → Message::AgentTokenUsage
  → App.token_usage → status_bar render
```

## Changes

### 1. Python: Track and report token usage

**`agents/multi_agent/state.py`** — Add two counter fields to `SharedState`:
```python
total_input_tokens: int = 0
total_output_tokens: int = 0
```

**`agents/multi_agent/bridge.py`** — Add `send_token_usage()` convenience function:
```python
def send_token_usage(input_tokens: int, output_tokens: int) -> None:
    send_to_tui({"type": "token_usage", "input_tokens": input_tokens, "output_tokens": output_tokens})
```

**`agents/multi_agent/models.py`** — Add `record_usage()` helper (next to `AgentDeps`):
```python
def record_usage(deps: AgentDeps, result) -> None:
    usage = result.usage()
    deps.state.total_input_tokens += usage.request_tokens or 0
    deps.state.total_output_tokens += usage.response_tokens or 0
    send_token_usage(deps.state.total_input_tokens, deps.state.total_output_tokens)
```

**Agent call sites** — Add `record_usage(deps, result)` after every `agent.run()` call (11 sites total):

| File | Function | Line | Notes |
|------|----------|------|-------|
| `agents/multi_agent/agents/user_agent.py` | `run_once` | ~109 | result already captured |
| `agents/multi_agent/agents/market_agent.py` | `run_on_user_request` | ~155 | result already captured |
| `agents/multi_agent/agents/market_agent.py` | `run_on_ticker` | ~174 | result already captured |
| `agents/multi_agent/agents/market_agent.py` | `run_on_consultation` | ~191 | result already captured |
| `agents/multi_agent/agents/ideation_agent.py` | `run_periodic` | ~215 | result already captured |
| `agents/multi_agent/agents/risk_agent.py` | `run_on_trade_idea` | ~205 | result already captured |
| `agents/multi_agent/agents/risk_agent.py` | `run_on_market_analysis` | ~222 | result already captured |
| `agents/multi_agent/agents/risk_agent.py` | `run_on_execution_update` | ~234 | result already captured |
| `agents/multi_agent/agents/risk_agent.py` | `run_position_review` | ~252 | result already captured |
| `agents/multi_agent/agents/execution_agent.py` | `run_on_approved_order` | ~123 | **must capture result** (currently discarded) |
| `agents/multi_agent/agents/execution_agent.py` | `run_on_close_position` | ~140 | **must capture result** (currently discarded) |

### 2. Rust: Receive token usage via IPC

**`src/agent.rs`** — Add variant to `AgentToTui` enum (line 60):
```rust
TokenUsage {
    input_tokens: u64,
    output_tokens: u64,
},
```

Handle it in `spawn_stdout_reader` (around line 248):
```rust
Ok(AgentToTui::TokenUsage { input_tokens, output_tokens }) => {
    let _ = tx.try_send(Message::AgentTokenUsage { input_tokens, output_tokens });
}
```

**`src/tui/event.rs`** — Add `AgentTokenUsage` variant to `Message` enum (line ~96) and handle in `update()`:
```rust
Message::AgentTokenUsage { input_tokens, output_tokens } => {
    app.token_usage.input_tokens = input_tokens;
    app.token_usage.output_tokens = output_tokens;
    None
}
```

### 3. Rust: App state for token usage

**`src/tui/app.rs`** — Add `TokenUsageStats` struct (follows `SimulationStats` pattern):
```rust
#[derive(Clone, Debug, Default)]
pub struct TokenUsageStats {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub input_cost_per_million: Option<Decimal>,
    pub output_cost_per_million: Option<Decimal>,
}
```
With `total_tokens()` and `estimated_cost()` methods.

Add `pub token_usage: TokenUsageStats` field to `App` struct (in Agent State section). Initialize as `TokenUsageStats::default()` in `App::new()`.

### 4. Rust: Config for cost rates

**`src/config.rs`** — Add optional cost fields to `AppConfig`:
```rust
pub token_input_cost: Option<Decimal>,
pub token_output_cost: Option<Decimal>,
```

Parse from env vars `LEESON_TOKEN_INPUT_COST` and `LEESON_TOKEN_OUTPUT_COST` in `fetch_config()`. Values are USD per million tokens (e.g. `0.03` = $0.03/1M tokens).

**`src/main.rs`** — Wire config to app state after `App::new()` (line ~75):
```rust
app.token_usage.input_cost_per_million = app_config.token_input_cost;
app.token_usage.output_cost_per_million = app_config.token_output_cost;
```

### 5. Rust: Render in status bar

**`src/tui/components/status_bar.rs`** — Add token usage display between sim spans and connection status, using magenta color. Shows compact count (e.g. `1.2k tok`, `3.5M tok`) and optional cost (`$0.0042`). Only shown when `total_tokens() > 0`.

## Design Decisions

- **Cumulative totals (not deltas)**: Python sends running total; Rust assigns (idempotent, no missed-message drift)
- **Env vars for cost config**: Follows existing `LEESON_SIMULATION` pattern — no new config files
- **Separate input/output rates**: Models price them differently
- **Magenta color**: Visually distinct from green (connection), cyan (balance), yellow (sim), red (errors)

## Verification

1. `cargo check` — compiles without errors
2. `cargo test` — existing tests pass
3. `cargo clippy` — no new warnings
4. Manual: run with `LEESON_TOKEN_INPUT_COST=0.03 LEESON_TOKEN_OUTPUT_COST=0.12` — verify status bar shows token count and cost after agent calls


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.claude/projects/-Users-denny-Documents-repos-leeson/61ce34ef-cd95-48a5-9b41-6d9bd3caac69.jsonl

## Summary

All done. Here's a summary of what was implemented:

**Python (agents layer):**
- `state.py` — Added `total_input_tokens` and `total_output_tokens` counters to `SharedState`
- `bridge.py` — Added `send_token_usage()` to emit cumulative totals to the TUI via JSON-lines
- `models.py` — Added `record_usage()` helper that extracts `request_tokens`/`response_tokens` from pydantic-ai `RunResult.usage()`, accumulates them, and reports to TUI
- All 5 agent files — Added `record_usage(deps, result)` after every `agent.run()` call (11 sites total). The two execution agent calls that previously discarded the result now capture it.

**Rust (TUI layer):**
- `agent.rs` — Added `TokenUsage { input_tokens, output_tokens }` variant to `AgentToTui` enum and handling in `spawn_stdout_reader`
- `event.rs` — Added `AgentTokenUsage` variant to `Message` enum with idempotent assignment in `update()`
- `app.rs` — Added `TokenUsageStats` struct with `total_tokens()` and `estimated_cost()` methods, and `token_usage` field on `App`
- `config.rs` — Added `token_input_cost` / `token_output_cost` fields parsed from `LEESON_TOKEN_INPUT_COST` / `LEESON_TOKEN_OUTPUT_COST` env vars (USD per million tokens)
- `main.rs` — Wired config cost rates into `app.token_usage`
- `status_bar.rs` — Added magenta token usage badge (e.g. `1.2k tok $0.0042`) between sim stats and connection status, only shown when tokens > 0

## Key Actions

- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/state.py
- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/bridge.py
- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/models.py
- - **Read**: /Users/denny/Documents/repos/leeson/src/agent.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/app.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/config.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/components/status_bar.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/agents/user_agent.py
