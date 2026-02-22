# Session Context

**Session ID:** 6b40c31e-6f8b-434e-b2b0-e3971f7b0f9c

**Commit Message:** Implement the following plan:

# Plan: Global Risk Parameters Overlay in

## Prompt

Implement the following plan:

# Plan: Global Risk Parameters Overlay in TUI

## Context

The trading platform currently has Rust-enforced risk limits (`risk.json`) but no way for the operator to define advisory risk parameters from the TUI. The operator needs to set global risk parameters — trades per month, intraday flag, trade size in EUR, and stop-loss in EUR — that apply across all trading pairs. These parameters are communicated to Python trading agents (not enforced by the Rust risk guard).

## Parameters

| Parameter          | Type      | Description                                   |
|--------------------|-----------|-----------------------------------------------|
| `trades_per_month` | `u32`     | Maximum trades allowed per calendar month     |
| `intraday`         | `bool`    | Whether intraday trading is permitted         |
| `trade_size_eur`   | `Decimal` | Standard trade size in EUR                    |
| `stop_loss_eur`    | `Decimal` | Maximum loss per trade in absolute EUR        |

## Approach

- Centered overlay/modal opened with `r` key in normal mode (follows existing confirm overlay pattern)
- Navigate fields with `j`/`k`, toggle bool with `Space`, edit numeric fields with `Enter` → type → `Enter`
- `s` saves and closes, `Esc` cancels (two-stage: first cancels field edit, then closes overlay)
- Persisted to `agent_risk.json` (separate from `risk.json` which lacks `Serialize` and has a different lifecycle)
- On save, updated parameters are re-sent to all running agents via `AgentCommand::RiskLimits`

## File Changes

### 1. `src/risk/config.rs` — Add `AgentRiskParams` struct

- New struct with `Serialize`, `Deserialize`, `Clone`, `Debug`
- `Default` impl with conservative values (10 trades/month, no intraday, 100 EUR trade size, 20 EUR stop-loss)
- `load(path)` — reads from JSON file, returns `Default` if file missing
- `save(path)` — writes pretty-printed JSON
- `describe()` — human-readable string for agent system prompts
- Unit tests for round-trip, defaults, missing file, describe output

### 2. `src/tui/app.rs` — Add editing state

- Add `Mode::RiskEdit` variant to `Mode` enum
- Add `RiskEditState` struct (selected field index, editing flag, input buffer, cursor, working copy of `AgentRiskParams`)
- Add to `App`: `agent_risk_params: AgentRiskParams` and `risk_edit: Option<RiskEditState>`
- Initialize both in `App::new()` (params = default, risk_edit = None)

### 3. `src/tui/event.rs` — Key handling and action

- Add `Action::SaveRiskParams(AgentRiskParams)` variant
- Add `'r'` key in `handle_normal_mode()` global section — opens overlay, sets `Mode::RiskEdit`
- Modify `Esc` handler for two-stage exit in `RiskEdit` mode
- Add `Mode::RiskEdit => handle_risk_edit_mode(app, key)` dispatch in `handle_key()`
- Implement `handle_risk_edit_mode()` — j/k navigate, Space toggles bool, Enter/i starts field edit, s saves
- Implement `handle_risk_field_edit()` — char input (digits + '.'), Backspace, Enter to confirm, Esc to cancel
- Update keybindings help text to mention `[r]risk`

### 4. `src/tui/ui.rs` — Render overlay

- Add `render_risk_edit_overlay()` following existing `render_confirm_overlay()` pattern
- Call from `render()` when `Mode::RiskEdit && risk_edit.is_some()`
- Shows title, 4 labeled fields with selection highlight, edit indicator, and keybinding help

### 5. `src/main.rs` — Wire up persistence and agent communication

- Load `AgentRiskParams` at startup (after risk_guard creation)
- Store in `app.agent_risk_params`
- On `AgentReady`: append `app.agent_risk_params.describe()` to risk limits sent to agent
- Handle `Action::SaveRiskParams`: persist to `agent_risk.json`, re-send combined limits to all agents

## Verification

1. `cargo test` — all existing + new unit tests pass
2. `cargo clippy` — no warnings
3. `cargo run` — TUI starts, press `r` → overlay appears with default values
4. Navigate with `j`/`k`, toggle intraday with `Space`, edit numeric fields with `Enter`
5. Press `s` to save → `agent_risk.json` file created with correct values
6. Restart app → values persist from `agent_risk.json`
7. Press `Esc` → overlay closes without saving changes


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.REDACTED.jsonl

## Summary

All done. Here's a summary of what was implemented:

### Files changed

| File | Changes |
|---|---|
| `src/risk/config.rs` | Added `AgentRiskParams` struct with `Serialize`/`Deserialize`/`Clone`/`Debug`, `Default` (10 trades/month, no intraday, 100 EUR trade size, 20 EUR stop-loss), `load()`/`save()`/`describe()` methods, and 4 unit tests |
| `src/tui/app.rs` | Added `Mode::RiskEdit` variant, `RiskEditState` struct (selected field, editing flag, input buffer, cursor, working copy), `agent_risk_params` and `risk_edit` fields on `App` |
| `src/tui/event.rs` | Added `Action::SaveRiskParams(AgentRiskParams)`, `r` key in `handle_normal_mode()` to open overlay, `handle_risk_edit_mode()` (j/k nav, Space toggle, Enter edit, s save, Esc cancel), `handle_risk_field_edit()` (digit/dot input, Backspace, Enter commit, Esc cancel edit), two-stage Esc handling |
| `src/tui/ui.rs` | Added `render_risk_edit_overlay()` following existing confirm overlay pattern, called from `render()` when `Mode::RiskEdit` |
| `src/tui/tabs/agent.rs` | Updated keybindings help to include `[r]risk` and `Mode::RiskEdit` help text |
| `src/tui/tabs/trading_pair.rs` | Updated keybindings to show risk edit help when in `Mode::RiskEdit`, added `[r]risk` to normal mode |
| `src/main.rs` | Loads `AgentRiskParams` from `agent_risk.json` at startup, includes `describe()` in risk limits sent on `AgentReady`, handles `Action::SaveRiskParams` to persist and re-send to all agents |
| `Cargo.toml` | Added `tempfile` dev dependency (for round-trip test) |

## Key Actions

- - **Task**: Explore codebase structure
- - **Read**: /Users/denny/Documents/repos/leeson/src/risk/config.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/app.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/ui.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/agent.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/tabs/agent.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/risk/mod.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/tabs/trading_pair.rs
