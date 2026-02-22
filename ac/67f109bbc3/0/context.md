# Session Context

**Session ID:** 6988f65f-9b30-402f-98c7-fd2f506383ac

**Commit Message:** Implement the following plan:

# Enhance Ideation Agent: Frequent Market

## Prompt

Implement the following plan:

# Enhance Ideation Agent: Frequent Market Checks & Trade Validation

## Context

The Ideation Agent currently runs on a fixed 15-minute cycle, fetching OHLC data and running a full technical analysis. It has no awareness of real-time ticker prices between cycles, and no validation to prevent proposing trades that duplicate or conflict with existing positions. The user wants the agent to check the market more frequently, generate ideas based on current conditions, and validate ideas against open trades.

## Approach: Dual-Cadence Loop + Trade Validation

Split the ideation loop into two cadences sharing the same LLM conversation history:

| Cadence | Interval | Data | Purpose |
|---------|----------|------|---------|
| Full OHLC analysis | 15 min (unchanged) | Kraken REST API + indicators | Deep technical analysis, pattern detection |
| Market pulse check | 2.5 min | SharedState tickers + positions | Spot price moves at key levels, validate existing ideas |

Add programmatic validation to `send_trade_idea` to block duplicate trades and warn on conflicting positions.

## Changes

### 1. `agents/multi_agent/agents/ideation_agent.py`

**A. Enhance `dynamic_context`** — Inject live ticker data and positions with explicit validation guidance:
- Add current ticker snapshots (last, bid, ask, volume) for all active pairs
- Add instruction: "Do NOT propose trades that duplicate existing positions"

**B. Add validation to `send_trade_idea` tool** — Before sending to Risk Agent:
- **Block** if same symbol + same side position already open (return descriptive error to LLM)
- **Warn** (but allow) if same symbol + opposite side position exists (legitimate reversal)
- No position → send normally

**C. Add `run_market_pulse()` function** — Lightweight fast-cadence check:
- Uses only SharedState data (no REST API calls)
- Compact prompt with ticker prices + open positions
- Asks: did price move to key levels? Any positions at risk? Urgent entry opportunity?
- Explicit instruction: "If nothing notable, say so briefly"

**D. Update system prompt** — Add guidance for pulse checks:
- "Check current market prices between full analyses to spot urgent opportunities at key levels"
- "During pulse checks, rely on key levels from your most recent full analysis"

### 2. `agents/multi_agent/orchestrator.py`

**Replace `_run_ideation_loop`** with a dual-cadence loop:
- Add `MARKET_PULSE_INTERVAL = 150` (2.5 minutes)
- Use a cycle counter: every ~6 pulse cycles, run a full OHLC analysis instead
- Same shared history for both cadences (LLM context carries across)
- Force full analysis on startup and after pair re-selection

### 3. `agents/multi_agent/agents/market_agent.py`

**Apply same `send_trade_idea` validation** — For consistency, add the same duplicate/conflict check to the Market Agent's `send_trade_idea` tool.

### Files unchanged

- `state.py` — Already has `tickers`, `positions`, `active_pairs`
- `models.py` — `TradeIdea` model unchanged
- `bus.py`, `bridge.py` — No changes needed
- `risk_agent.py` — Still the final gatekeeper; upstream validation is a pre-filter
- `technical.py`, `execution_agent.py` — No changes

## Implementation Order

1. Enhance `dynamic_context` in `ideation_agent.py`
2. Add validation to `send_trade_idea` in both `ideation_agent.py` and `market_agent.py`
3. Add `run_market_pulse()` to `ideation_agent.py`
4. Update system prompt in `ideation_agent.py`
5. Replace `_run_ideation_loop` in `orchestrator.py` with dual-cadence loop

## Verification

1. `cargo check` — Rust side unaffected but confirm no build issues
2. `python -c "from multi_agent.agents.ideation_agent import run_market_pulse, run_periodic"` — Verify imports work
3. Manual test: run the system with active pairs, observe:
   - Full OHLC analysis fires on startup
   - Pulse checks fire every ~2.5 min between full analyses
   - Duplicate trade proposals are blocked with descriptive message
   - Opposite-side proposals produce a warning but still reach Risk Agent


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.REDACTED.jsonl

## Summary

All tasks complete. Here's a summary of what was implemented:

### Changes Made

**1. `agents/multi_agent/models.py`** — Added `validate_trade_idea()` shared helper
- Blocks duplicate trades (same symbol + same side as existing position)
- Warns but allows opposite-side trades (legitimate reversals)
- Returns `(ok, message)` tuple for both agents to use

**2. `agents/multi_agent/agents/ideation_agent.py`**
- **Enhanced `dynamic_context`** — Now injects live ticker snapshots (last, bid, ask, volume) for all active pairs, plus explicit instruction not to propose duplicate trades
- **Added validation to `send_trade_idea`** — Calls `validate_trade_idea()` before sending to Risk Agent; blocked ideas return an error message to the LLM
- **Updated system prompt** — Added guidance for pulse checks (rely on key levels from full analysis, don't force trades)
- **Added `run_market_pulse()`** — Lightweight function that uses only SharedState tickers/positions (no REST calls), asks the LLM to assess price movement at key levels, position risk, and urgent opportunities

**3. `agents/multi_agent/agents/market_agent.py`**
- **Added same validation to `send_trade_idea`** — Consistent duplicate/conflict checking

**4. `agents/multi_agent/orchestrator.py`**
- **Added `MARKET_PULSE_INTERVAL = 150`** (2.5 min) and `PULSES_PER_FULL_CYCLE = 6`
- **Replaced `_run_ideation_loop`** with dual-cadence loop: full OHLC at `pulse_count == 0`, market pulse otherwise, cycling every 6 intervals. Resets to full analysis on startup and after pair re-selection.

## Key Actions

- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/agents/ideation_agent.py
- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/orchestrator.py
- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/agents/market_agent.py
- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/state.py
- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/models.py
- - **TaskCreate**: Inject live ticker data and positions with explicit validation guidance into the ideation agent's dynamic context.
- - **TaskCreate**: Add programmatic validation to block duplicate trades (same symbol + same side) and warn on conflicting positions (same symbol + opposite side).
- - **TaskCreate**: Apply same duplicate/conflict validation to Market Agent's send_trade_idea tool for consistency.
- - **TaskCreate**: Add a lightweight fast-cadence check function that uses SharedState data (no REST calls), compact prompt with ticker prices + open positions, asks about key levels, at-risk positions, urgent opportunities.
- - **TaskCreate**: Add guidance for pulse checks: check current market prices between full analyses to spot urgent opportunities at key levels, rely on key levels from most recent full analysis during pulse checks.
