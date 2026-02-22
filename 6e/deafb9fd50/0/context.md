# Session Context

**Session ID:** f7cf7a41-2190-40dc-abd9-00ee3860dab2

**Commit Message:** Implement the following plan:

# Plan: Add Technical Analysis Indicators

## Prompt

Implement the following plan:

# Plan: Add Technical Analysis Indicators to Ideation Agent

## Context

The ideation agent currently fetches raw OHLC candle data from Kraken and passes it to the LLM for "visual" analysis of the candle table. There are no computed indicators — the LLM must infer trends, momentum, and levels from raw price numbers alone. Adding calculated indicators (RSI, MACD, EMAs, Bollinger Bands, ATR, support/resistance) will give the agent quantitative signals to work with, leading to better-calibrated trade ideas.

## Approach: Pure Python, No New Dependencies

The Kraken API returns at most 720 candles. At this scale, plain Python lists and arithmetic are perfectly adequate. Every indicator needed is a straightforward formula (10-20 lines each). This avoids adding `pandas`/`numpy`/`ta-lib` to a project that currently has only 3 Python dependencies.

## Files to Create/Modify

| File | Action |
|------|--------|
| `agents/multi_agent/technical.py` | **Create** — all indicator calculations |
| `agents/multi_agent/agents/ideation_agent.py` | **Modify** — refactor fetch, add tools, update prompt |

## Step 1: Create `agents/multi_agent/technical.py`

New module with pure functions operating on lists — no I/O, no side effects.

**Data type:**
- `Candle` NamedTuple: `(time, open, high, low, close, vwap, volume, count)`
- `parse_candles(raw_list) -> list[Candle]` to convert Kraken arrays

**Indicator functions** (all return `list[float | None]`, `None` for insufficient data):

| Function | Parameters | Purpose |
|----------|-----------|---------|
| `sma(closes, period)` | period=20 | Simple Moving Average |
| `ema(closes, period)` | period=9, 21 | Exponential Moving Average |
| `rsi(closes, period)` | period=14 | Relative Strength Index (Wilder's smoothing) |
| `macd(closes, fast, slow, signal)` | 12, 26, 9 | MACD line, signal line, histogram |
| `bollinger_bands(closes, period, num_std)` | 20, 2.0 | Upper, middle, lower bands |
| `atr(candles, period)` | period=14 | Average True Range (Wilder's smoothing) |
| `volume_sma(volumes, period)` | period=20 | Volume moving average for relative volume |
| `price_momentum(closes, period)` | period=10 | % price change over N candles |

**Key levels detection:**
- `find_key_levels(candles, lookback=5, tolerance_pct=0.5)` — swing high/low detection
- Returns `{"support": [...], "resistance": [...]}` sorted by proximity to current price
- Clusters nearby levels within tolerance, returns top 3 of each

**Formatting:**
- `compute_all(candles) -> str` — runs all indicators and formats a compact text summary

Output format (per pair, ~300 tokens):
```
=== Technical Indicators (60min) ===
TREND: EMA(9): 97,234  EMA(21): 96,891  Momentum(10): +1.8%
RSI(14): 62.3 (neutral-bullish)
MACD: line=+185.3  signal=+142.1  histogram=+43.2 (bullish, expanding)
BOLLINGER(20,2): upper=98,100  mid=96,750  lower=95,400  position=72%
ATR(14): 412.3 (0.42% of price)
VOLUME: current=125.4  avg(20)=98.2  relative=1.28x
KEY LEVELS: R: 98,500 | 99,200 | 100,000  S: 96,200 | 95,800 | 94,500
```

## Step 2: Refactor `fetch_ohlc` in `ideation_agent.py`

Split the current `fetch_ohlc` into two functions:

1. `_fetch_raw_ohlc(symbol, interval) -> list[list] | str` — HTTP fetch, returns raw candle arrays (or error string)
2. `_format_ohlc(symbol, raw_candles, interval) -> str` — formats the 24-candle table (extracted from current code)

The existing `fetch_ohlc` delegates to both. Backward compatible — `get_ohlc` tool and existing callers unchanged.

**Critical**: Use ALL returned candles (up to 720) for indicator calculation, not just the 24 displayed. This ensures MACD (needs 35+ candles) and other indicators have enough data.

## Step 3: Add New Tools to Ideation Agent

**Tool 1: `calculate_indicators(symbol, interval=60)`**
- Fetches raw OHLC via `_fetch_raw_ohlc`
- Parses candles, runs `compute_all()`, returns formatted text
- Allows the LLM to get indicators for any timeframe on demand

**Tool 2: `find_support_resistance(symbol, interval=60)`**
- Fetches raw OHLC via `_fetch_raw_ohlc`
- Parses candles, runs `find_key_levels()`, returns formatted levels
- Useful for longer timeframes (240min, 1440min) for more significant levels

## Step 4: Update `run_periodic()`

Change the periodic run to automatically include computed indicators alongside raw OHLC:

1. Use `_fetch_raw_ohlc` instead of `fetch_ohlc`
2. For each pair, compute both `_format_ohlc` (24-candle table) and `compute_all` (indicators on full dataset)
3. Update the prompt to reference the indicators and guide the LLM's analysis

## Step 5: Update System Prompt

Expand to reflect quantitative capabilities:
- Reference specific indicators and their interpretation
- Guide the agent to use multiple confirming indicators before proposing trades
- Mention the new tools (`calculate_indicators`, `find_support_resistance`)

## Verification

1. `python -c "from multi_agent.technical import compute_all, parse_candles, find_key_levels"` — imports work
2. Run the full agent system with `cargo run` and verify ideation agent output includes indicator summaries in Panel 1
3. Check that indicator values appear reasonable (RSI 0-100, MACD signs match price direction, key levels bracket current price)


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.claude/projects/-Users-denny-Documents-repos-leeson/53ce414c-453b-4cd5-90c0-8285c34df0ba.jsonl

## Summary

All done. Here's a summary of what was implemented:

**Created `agents/multi_agent/technical.py`** — Pure Python indicator module with:
- `Candle` NamedTuple and `parse_candles()` for Kraken data
- 8 indicator functions: `sma`, `ema`, `rsi`, `macd`, `bollinger_bands`, `atr`, `volume_sma`, `price_momentum`
- `find_key_levels()` — swing high/low detection with clustering
- `compute_all()` — formats all indicators into a compact ~300 token summary

**Modified `agents/multi_agent/agents/ideation_agent.py`**:
- Refactored `fetch_ohlc` into `_fetch_raw_ohlc` (returns raw arrays) and `_format_ohlc` (24-candle table), with `fetch_ohlc` as backward-compatible wrapper
- Added `calculate_indicators` tool — computes all indicators on full dataset (up to 720 candles)
- Added `find_support_resistance_tool` — key levels for any timeframe
- Updated `run_periodic()` to include computed indicators alongside the candle table
- Updated system prompt to reference quantitative indicators and require 2-3 confirming signals before proposing trades

## Key Actions

- - **Read**: /Users/denny/Documents/repos/leeson/agents/multi_agent/agents/ideation_agent.py
- - **Glob**: agents/multi_agent/technical.py
- - **Glob**: agents/multi_agent/*.py
- - **TaskCreate**: Create agents/multi_agent/technical.py with pure Python indicator functions: Candle NamedTuple, parse_candles, sma, ema, rsi, macd, bollinger_bands, atr, volume_sma, price_momentum, find_key_levels, and compute_all formatter.
- - **TaskCreate**: Refactor fetch_ohlc into _fetch_raw_ohlc and _format_ohlc, add calculate_indicators and find_support_resistance tools, update run_periodic to include indicators, and update system prompt.
- - **TaskCreate**: Verify the module imports correctly and the integration is sound.
- - **TaskUpdate**: 
- - **Write**: /Users/denny/Documents/repos/leeson/agents/multi_agent/technical.py
- - **TaskUpdate**: 
- - **TaskUpdate**: 
