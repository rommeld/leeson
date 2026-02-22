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
from multi_agent.models import AgentDeps, AgentRole, TradeIdea, record_usage

PANEL = 1

_KRAKEN_OHLC_URL = "https://api.kraken.com/0/public/OHLC"
_VALID_INTERVALS = {1, 5, 15, 30, 60, 240, 1440, 10080, 21600}


def _ws_pair_to_rest(symbol: str) -> str:
    """Convert WebSocket pair format to REST format ('BTC/USD' → 'BTCUSD')."""
    return symbol.replace("/", "")


ideation_agent = Agent(
    model=None,
    defer_model_check=True,
    deps_type=AgentDeps,
    system_prompt=(
        "You are the Ideation Agent for the Leeson crypto trading system. "
        "You are an experienced technical analyst focused on multi-hour and "
        "daily chart patterns.\n\n"
        "Your expertise:\n"
        "- Trend identification (higher highs/lows, moving average crossovers)\n"
        "- Support and resistance level detection\n"
        "- Candlestick pattern recognition (engulfing, doji, hammer, etc.)\n"
        "- Volume analysis and divergence detection\n"
        "- Multi-timeframe confluence\n\n"
        "Your role:\n"
        "- Analyze the OHLC candlestick data provided in each prompt\n"
        "- Use the get_ohlc tool for additional timeframes if deeper analysis is needed\n"
        "- Identify high-probability setups based on chart patterns and trends\n"
        "- Send trade ideas to Risk Agent using the send_trade_idea tool\n"
        "- Focus on swing/position trades (hours to days), not scalping\n\n"
        "You complement the Market Agent who focuses on real-time price action "
        "and microstructure. You focus on the bigger picture — trend direction, "
        "key levels, and pattern-based entries.\n\n"
        "Be concise. Only propose trades with clear technical justification "
        "and calibrated probability scores."
    ),
)


@ideation_agent.instructions
async def dynamic_context(ctx: RunContext[AgentDeps]) -> str:
    """Inject current position info for context."""
    state = ctx.deps.state
    parts = []
    if state.positions:
        parts.append("Open positions:")
        for p in state.positions.values():
            parts.append(
                f"  {p.symbol} {p.side} qty={p.qty} "
                f"entry={p.entry_price} current={p.current_price} "
                f"pnl={p.unrealized_pnl}"
            )
    return "\n".join(parts) if parts else "No open positions."


async def fetch_ohlc(symbol: str, interval: int = 60) -> str:
    """Fetch and format OHLC candlestick data from Kraken.

    Standalone function that can be called both programmatically (for
    pre-fetching) and from the LLM tool.

    Args:
        symbol: Trading pair (e.g. "BTC/USD").
        interval: Candle interval in minutes.
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
    # The result contains the pair data under a Kraken-specific key
    # and a "last" timestamp — find the candle array
    candles = None
    for key, value in result.items():
        if key != "last" and isinstance(value, list):
            candles = value
            break

    if not candles:
        return f"No OHLC data returned for {symbol}"

    total = len(candles)
    recent = candles[-24:] if len(candles) >= 24 else candles

    # Each candle: [time, open, high, low, close, vwap, volume, count]
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
    output_to_panel(
        PANEL,
        f"[ideation] {symbol} {side} qty={suggested_qty} p={probability:.0%} — {reason}",
    )
    return f"Trade idea sent to Risk Agent: {symbol} {side}"


async def run_periodic(
    deps: AgentDeps, history: list, *, model: object
) -> list:
    """Fetch OHLC data for all active pairs and analyze for opportunities."""
    pairs = deps.state.active_pairs
    if not pairs:
        return history

    # Pre-fetch hourly OHLC data for all active pairs concurrently.
    ohlc_results = await asyncio.gather(
        *(fetch_ohlc(pair) for pair in pairs),
        return_exceptions=True,
    )

    ohlc_sections: list[str] = []
    for pair, result in zip(pairs, ohlc_results):
        if isinstance(result, Exception):
            ohlc_sections.append(f"--- {pair} ---\nFetch error: {result}")
        else:
            ohlc_sections.append(f"--- {pair} ---\n{result}")

    ohlc_block = "\n\n".join(ohlc_sections)

    prompt = (
        f"Analyze the following active pairs for swing trade opportunities.\n\n"
        f"Hourly OHLC data has already been fetched for each pair:\n\n"
        f"{ohlc_block}\n\n"
        f"For each pair, assess trend direction, key support/resistance levels, "
        f"and any notable candlestick patterns. If you identify a high-probability "
        f"setup, use send_trade_idea to propose it. If no clear opportunity exists, "
        f"briefly summarize the market structure. You can use the get_ohlc tool to "
        f"fetch additional timeframes (e.g. 15-min, daily) if deeper analysis is needed."
    )

    result = await ideation_agent.run(
        prompt, deps=deps, message_history=history, model=model
    )
    record_usage(deps, result)
    history = result.all_messages()[-30:]
    output_to_panel(PANEL, f"[ideation] {result.output}")
    return history
