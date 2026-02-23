"""Ideation Agent — longer-timeframe technical analysis via OHLC data (Panel 1).

Complements the Market Agent's real-time focus by analyzing 24+ hours of
candlestick data to identify trends, support/resistance levels, and
pattern-based trading opportunities. Sends trade ideas to the Risk Agent
through the same approval flow as the Market Agent.
"""

from __future__ import annotations

import asyncio
from datetime import UTC, datetime

import httpx
from pydantic_ai import Agent, RunContext

from multi_agent.bridge import output_to_panel
from multi_agent.models import (
    AgentDeps,
    AgentRole,
    TradeIdea,
    run_agent_streamed,
    validate_trade_idea,
)
from multi_agent.technical import compute_all, find_key_levels, parse_candles

PANEL = 1

_KRAKEN_OHLC_URL = "https://api.kraken.com/0/public/OHLC"
_VALID_INTERVALS = {1, 5, 15, 30, 60, 240, 1440, 10080, 21600}


def _ws_pair_to_rest(symbol: str) -> str:
    """Convert WebSocket pair format to REST format ('BTC/USD' → 'BTCUSD')."""
    return symbol.replace("/", "")


# ---------------------------------------------------------------------------
# OHLC fetch / format helpers
# ---------------------------------------------------------------------------


async def _fetch_raw_ohlc(
    symbol: str, interval: int = 60
) -> list[list] | str:
    """Fetch raw OHLC candle arrays from Kraken.

    Returns the raw list of candle arrays on success, or an error string.
    """
    if interval not in _VALID_INTERVALS:
        return (
            f"Invalid interval {interval}. "
            f"Valid intervals: {sorted(_VALID_INTERVALS)}"
        )

    rest_pair = _ws_pair_to_rest(symbol)

    try:
        async with httpx.AsyncClient(timeout=10.0) as client:
            resp = await client.get(
                _KRAKEN_OHLC_URL,
                params={"pair": rest_pair, "interval": interval},
            )
            resp.raise_for_status()
            data = resp.json()
    except httpx.HTTPStatusError as exc:
        return f"Kraken API error: HTTP {exc.response.status_code}"
    except httpx.RequestError as exc:
        return f"Request failed: {exc}"

    errors = data.get("error", [])
    if errors:
        return f"Kraken API error: {', '.join(errors)}"

    result = data.get("result", {})
    for key, value in result.items():
        if key != "last" and isinstance(value, list):
            return value

    return f"No OHLC data returned for {symbol}"


def _format_ohlc(symbol: str, raw_candles: list[list], interval: int = 60) -> str:
    """Format raw candle arrays into the 24-candle summary table."""
    total = len(raw_candles)
    recent = raw_candles[-24:] if total >= 24 else raw_candles

    first_close = float(recent[0][4])
    last_close = float(recent[-1][4])
    period_high = max(float(c[2]) for c in recent)
    period_low = min(float(c[3]) for c in recent)
    total_volume = sum(float(c[6]) for c in recent)
    change = last_close - first_close
    change_pct = (change / first_close * 100) if first_close else 0

    lines = [
        f"OHLC for {symbol} (interval={interval}min, {total} candles available)",
        f"Latest close: {last_close}  |  Period high: {period_high}  |  Period low: {period_low}",
        f"24-candle change: {change:+.2f} ({change_pct:+.2f}%)  |  Total volume: {total_volume:.2f}",
        "",
        "Recent 24 candles (newest last):",
        f"{'Time':<20} | {'Open':>10} | {'High':>10} | {'Low':>10} | {'Close':>10} | {'Volume':>10}",
        "-" * 85,
    ]

    for c in recent:
        ts = datetime.fromtimestamp(int(c[0]), tz=UTC).strftime("%Y-%m-%d %H:%M")
        lines.append(
            f"{ts:<20} | {float(c[1]):>10.1f} | {float(c[2]):>10.1f} | "
            f"{float(c[3]):>10.1f} | {float(c[4]):>10.1f} | {float(c[6]):>10.2f}"
        )

    return "\n".join(lines)


async def fetch_ohlc(symbol: str, interval: int = 60) -> str:
    """Fetch and format OHLC candlestick data from Kraken.

    Standalone function that can be called both programmatically (for
    pre-fetching) and from the LLM tool.
    """
    raw = await _fetch_raw_ohlc(symbol, interval)
    if isinstance(raw, str):
        return raw
    return _format_ohlc(symbol, raw, interval)


# ---------------------------------------------------------------------------
# Agent definition
# ---------------------------------------------------------------------------

ideation_agent = Agent(
    model=None,
    defer_model_check=True,
    deps_type=AgentDeps,
    system_prompt=(
        "You are the Ideation Agent for the Leeson crypto trading system. "
        "You are an experienced quantitative technical analyst focused on "
        "multi-hour and daily chart patterns.\n\n"
        "Your expertise:\n"
        "- Trend identification via EMA crossovers and price momentum\n"
        "- RSI divergence and overbought/oversold conditions\n"
        "- MACD signal line crossovers and histogram momentum\n"
        "- Bollinger Band squeeze and breakout detection\n"
        "- ATR-based volatility assessment for position sizing\n"
        "- Support and resistance level detection from swing highs/lows\n"
        "- Candlestick pattern recognition (engulfing, doji, hammer, etc.)\n"
        "- Volume analysis and divergence detection\n"
        "- Multi-timeframe confluence\n\n"
        "Your role:\n"
        "- Analyze the OHLC data and computed technical indicators provided "
        "in each prompt\n"
        "- Use the calculate_indicators tool for additional timeframes\n"
        "- Use the find_support_resistance tool for longer-timeframe levels "
        "(e.g. 240min, 1440min)\n"
        "- Use the get_ohlc tool for raw candle data when deeper visual "
        "analysis is needed\n"
        "- Require at least 2-3 confirming indicators before proposing a "
        "trade (e.g. RSI + MACD alignment, or EMA crossover + Bollinger "
        "breakout)\n"
        "- Send trade ideas to Risk Agent using the send_trade_idea tool\n"
        "- Focus on swing/position trades (hours to days), not scalping\n\n"
        "You complement the Market Agent who focuses on real-time price "
        "action and microstructure. You focus on the bigger picture — trend "
        "direction, key levels, and indicator-confirmed entries.\n\n"
        "Between full OHLC analyses, you receive market pulse checks with "
        "current ticker prices and open positions. During pulse checks:\n"
        "- Rely on key levels and trends from your most recent full analysis\n"
        "- Spot price moves approaching key support/resistance levels\n"
        "- Identify positions that may be at risk\n"
        "- Flag urgent entry opportunities if price reaches a level you "
        "previously identified\n"
        "- If nothing notable, say so briefly — do not force a trade idea\n\n"
        "Be concise. Only propose trades with clear technical justification, "
        "multiple confirming indicators, and calibrated probability scores."
    ),
)


@ideation_agent.instructions
async def dynamic_context(ctx: RunContext[AgentDeps]) -> str:
    """Inject live ticker data and open positions for context."""
    state = ctx.deps.state
    parts: list[str] = []

    # Live ticker snapshots for active pairs
    ticker_lines: list[str] = []
    for symbol in state.active_pairs:
        ticker = state.tickers.get(symbol)
        if ticker:
            ticker_lines.append(
                f"  {symbol}: last={ticker.last} bid={ticker.bid} "
                f"ask={ticker.ask} vol={ticker.volume}"
            )
    if ticker_lines:
        parts.append("Current ticker data:")
        parts.extend(ticker_lines)

    # Open positions
    if state.positions:
        parts.append("Open positions:")
        for p in state.positions.values():
            parts.append(
                f"  {p.symbol} {p.side} qty={p.qty} "
                f"entry={p.entry_price} current={p.current_price} "
                f"pnl={p.unrealized_pnl}"
            )
        parts.append(
            "\nIMPORTANT: Do NOT propose trades that duplicate existing "
            "positions (same symbol and same side). If you see a strong "
            "signal in the opposite direction of an existing position, "
            "you may propose a reversal trade but note the conflict."
        )
    else:
        parts.append("No open positions.")

    return "\n".join(parts)


# ---------------------------------------------------------------------------
# Tools
# ---------------------------------------------------------------------------


@ideation_agent.tool
async def get_ohlc(
    ctx: RunContext[AgentDeps],
    symbol: str,
    interval: int = 60,
) -> str:
    """Fetch OHLC candlestick data from Kraken for technical analysis.

    Use this tool to fetch additional timeframes beyond the hourly data
    already provided in the prompt.

    Args:
        symbol: Trading pair (e.g. "BTC/USD").
        interval: Candle interval in minutes. Valid: 1, 5, 15, 30, 60, 240, 1440, 10080, 21600.
    """
    return await fetch_ohlc(symbol, interval)


@ideation_agent.tool
async def calculate_indicators(
    ctx: RunContext[AgentDeps],
    symbol: str,
    interval: int = 60,
) -> str:
    """Calculate technical indicators (RSI, MACD, EMAs, Bollinger, ATR) for a pair.

    Uses all available candles (up to 720) for accurate indicator
    calculation. Use this for timeframes not already in the prompt.

    Args:
        symbol: Trading pair (e.g. "BTC/USD").
        interval: Candle interval in minutes. Valid: 1, 5, 15, 30, 60, 240, 1440, 10080, 21600.
    """
    raw = await _fetch_raw_ohlc(symbol, interval)
    if isinstance(raw, str):
        return raw
    candles = parse_candles(raw)
    return compute_all(candles, interval)


@ideation_agent.tool
async def find_support_resistance_tool(
    ctx: RunContext[AgentDeps],
    symbol: str,
    interval: int = 60,
) -> str:
    """Find key support and resistance levels for a trading pair.

    Useful for longer timeframes (240min, 1440min) to identify more
    significant levels.

    Args:
        symbol: Trading pair (e.g. "BTC/USD").
        interval: Candle interval in minutes. Valid: 1, 5, 15, 30, 60, 240, 1440, 10080, 21600.
    """
    raw = await _fetch_raw_ohlc(symbol, interval)
    if isinstance(raw, str):
        return raw

    candles = parse_candles(raw)
    current = candles[-1].close
    levels = find_key_levels(candles)

    r_str = " | ".join(f"{lv:,.1f}" for lv in sorted(levels["resistance"])) or "none detected"
    s_str = " | ".join(f"{lv:,.1f}" for lv in sorted(levels["support"])) or "none detected"

    return (
        f"Key levels for {symbol} ({interval}min, {len(candles)} candles)\n"
        f"Current price: {current:,.1f}\n"
        f"Resistance: {r_str}\n"
        f"Support: {s_str}"
    )


@ideation_agent.tool
async def send_trade_idea(
    ctx: RunContext[AgentDeps],
    symbol: str,
    side: str,
    reason: str,
    probability: float,
    suggested_qty: str,
    order_type: str = "limit",
    suggested_price: str | None = None,
) -> str:
    """Send a trade idea to the Risk Agent for evaluation.

    Args:
        symbol: Trading pair (e.g. "BTC/USD").
        side: "buy" or "sell".
        reason: Brief justification for the trade.
        probability: Confidence score from 0.0 to 1.0.
        suggested_qty: Suggested quantity.
        order_type: "limit" or "market".
        suggested_price: Limit price (required for limit orders).
    """
    # Validate against open positions
    ok, msg = validate_trade_idea(ctx.deps.state.positions, symbol, side)
    if not ok:
        output_to_panel(PANEL, f"[ideation] {msg}")
        return msg

    idea = TradeIdea(
        symbol=symbol,
        side=side,
        reason=reason,
        probability=probability,
        suggested_qty=suggested_qty,
        suggested_price=suggested_price,
        order_type=order_type,
    )
    await ctx.deps.bus.send(AgentRole.RISK, idea)

    warning = f" ({msg})" if msg else ""
    output_to_panel(
        PANEL,
        f"[ideation] {symbol} {side} qty={suggested_qty} p={probability:.0%} — {reason}{warning}",
    )
    return f"Trade idea sent to Risk Agent: {symbol} {side}{warning}"


# ---------------------------------------------------------------------------
# Periodic run
# ---------------------------------------------------------------------------


async def _fetch_pair_data(pair: str) -> tuple[str, list[list] | str]:
    """Fetch raw OHLC for a single pair, returning (pair, raw_or_error)."""
    raw = await _fetch_raw_ohlc(pair)
    return pair, raw


async def run_periodic(
    deps: AgentDeps, history: list, *, model: object
) -> list:
    """Fetch OHLC data for all active pairs and analyze for opportunities."""
    pairs = deps.state.active_pairs
    if not pairs:
        return history

    # Pre-fetch hourly OHLC data for all active pairs concurrently.
    results = await asyncio.gather(
        *(_fetch_pair_data(pair) for pair in pairs),
        return_exceptions=True,
    )

    ohlc_sections: list[str] = []
    for result in results:
        if isinstance(result, Exception):
            ohlc_sections.append(f"Fetch error: {result}")
            continue

        pair, raw = result
        if isinstance(raw, str):
            ohlc_sections.append(f"--- {pair} ---\n{raw}")
            continue

        # Format both the 24-candle table and computed indicators
        table = _format_ohlc(pair, raw)
        candles = parse_candles(raw)
        indicators = compute_all(candles)
        ohlc_sections.append(f"--- {pair} ---\n{table}\n\n{indicators}")

    ohlc_block = "\n\n".join(ohlc_sections)

    prompt = (
        f"Analyze the following active pairs for swing trade opportunities.\n\n"
        f"Hourly OHLC data and computed technical indicators for each pair:\n\n"
        f"{ohlc_block}\n\n"
        f"For each pair, assess:\n"
        f"1. Trend direction (EMA alignment, momentum)\n"
        f"2. Momentum conditions (RSI, MACD histogram direction)\n"
        f"3. Volatility context (Bollinger position, ATR)\n"
        f"4. Key support/resistance levels\n"
        f"5. Volume confirmation\n\n"
        f"Only propose a trade if 2-3 indicators confirm the setup. Use "
        f"send_trade_idea to propose trades. You can use calculate_indicators "
        f"or find_support_resistance for additional timeframes if you need "
        f"multi-timeframe confluence."
    )

    return await run_agent_streamed(
        ideation_agent, prompt, deps=deps, history=history, model=model, panel=PANEL
    )


async def run_market_pulse(
    deps: AgentDeps, history: list, *, model: object
) -> list:
    """Lightweight market pulse check using only SharedState data.

    No REST API calls — uses cached ticker snapshots and open positions
    to spot urgent opportunities or risks between full OHLC analyses.
    """
    state = deps.state
    pairs = state.active_pairs
    if not pairs:
        return history

    # Build compact ticker summary
    ticker_lines: list[str] = []
    for symbol in pairs:
        ticker = state.tickers.get(symbol)
        if ticker:
            ticker_lines.append(
                f"  {symbol}: last={ticker.last} bid={ticker.bid} "
                f"ask={ticker.ask} vol={ticker.volume}"
            )

    if not ticker_lines:
        return history  # no ticker data yet

    # Build position summary
    position_lines: list[str] = []
    for p in state.positions.values():
        position_lines.append(
            f"  {p.symbol} {p.side} qty={p.qty} "
            f"entry={p.entry_price} current={p.current_price} "
            f"pnl={p.unrealized_pnl}"
        )

    pos_block = "\n".join(position_lines) if position_lines else "  None"

    prompt = (
        "MARKET PULSE CHECK — Quick assessment of current prices.\n\n"
        f"Current prices:\n{chr(10).join(ticker_lines)}\n\n"
        f"Open positions:\n{pos_block}\n\n"
        "Based on your most recent full analysis, assess:\n"
        "1. Has price moved to any key support/resistance levels?\n"
        "2. Are any open positions at risk and need attention?\n"
        "3. Is there an urgent entry opportunity at a level you identified?\n\n"
        "If nothing notable, say so briefly. Do not force trade ideas. "
        "Use send_trade_idea only if you see a clear, urgent opportunity."
    )

    return await run_agent_streamed(
        ideation_agent, prompt, deps=deps, history=history, model=model, panel=PANEL
    )
