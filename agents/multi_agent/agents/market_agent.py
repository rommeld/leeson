"""Market Agent — technical analysis and trade ideas (Panel 1).

Analyzes market data, generates trade ideas with probability scores,
and responds to consultations from the Risk Agent. An experienced
creative trader with deep technical and microstructure knowledge.
"""

from __future__ import annotations

from pydantic_ai import Agent, RunContext

from multi_agent.bridge import output_to_panel
from multi_agent.models import (
    AgentDeps,
    AgentRole,
    ConsultMarket,
    MarketAnalysis,
    TradeIdea,
    UserRequest,
    run_agent_streamed,
    validate_trade_idea,
)

PANEL = 1

market_agent = Agent(
    model=None,
    defer_model_check=True,
    deps_type=AgentDeps,
    system_prompt=(
        "You are the Market Agent for the Leeson crypto trading system. "
        "You are an experienced, creative crypto trader with deep knowledge of:\n"
        "- Technical analysis (support/resistance, momentum, patterns)\n"
        "- Market microstructure (order flow, liquidity, spread dynamics)\n"
        "- Probability calibration (never overstate confidence)\n\n"
        "Your role:\n"
        "- Analyze market data and generate trade ideas\n"
        "- Assign honest probability scores (0.0-1.0) to each idea\n"
        "- Respond to Risk Agent consultations about existing positions\n"
        "- Use the send_trade_idea tool when you see a good opportunity\n\n"
        "Be concise. Focus on actionable insights, not verbose explanations. "
        "Always include your reasoning and a calibrated probability score."
    ),
)


@market_agent.instructions
async def dynamic_context(ctx: RunContext[AgentDeps]) -> str:
    """Inject current ticker data for active pairs."""
    state = ctx.deps.state
    parts = []
    for symbol in state.active_pairs:
        ticker = state.tickers.get(symbol)
        if ticker:
            parts.append(
                f"{symbol}: bid={ticker.bid} ask={ticker.ask} "
                f"last={ticker.last} vol={ticker.volume}"
            )
    if state.positions:
        parts.append("Open positions:")
        for p in state.positions.values():
            parts.append(
                f"  {p.symbol} {p.side} qty={p.qty} "
                f"entry={p.entry_price} current={p.current_price} "
                f"pnl={p.unrealized_pnl}"
            )
    return "\n".join(parts) if parts else "No market data yet."


@market_agent.tool
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
        output_to_panel(PANEL, f"[market] {msg}")
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
        f"[idea] {symbol} {side} qty={suggested_qty} p={probability:.0%} — {reason}{warning}",
    )
    return f"Trade idea sent to Risk Agent: {symbol} {side}{warning}"


@market_agent.tool
async def respond_to_consultation(
    ctx: RunContext[AgentDeps],
    symbol: str,
    analysis: str,
    recommendation: str,
) -> str:
    """Respond to a Risk Agent consultation about a position.

    Args:
        symbol: The trading pair being consulted about.
        analysis: Your analysis of the current situation.
        recommendation: One of "hold", "close", or "add".
    """
    response = MarketAnalysis(
        symbol=symbol, analysis=analysis, recommendation=recommendation
    )
    await ctx.deps.bus.send(AgentRole.RISK, response)
    return f"Consultation response sent: {symbol} → {recommendation}"


@market_agent.tool
async def get_ticker(ctx: RunContext[AgentDeps], symbol: str) -> str:
    """Get the latest ticker data for a symbol.

    Args:
        symbol: Trading pair to look up (e.g. "BTC/USD").
    """
    ticker = ctx.deps.state.tickers.get(symbol)
    if not ticker:
        return f"No ticker data for {symbol}"
    return (
        f"{symbol}: bid={ticker.bid} ask={ticker.ask} "
        f"last={ticker.last} vol={ticker.volume}"
    )


async def run_on_user_request(
    deps: AgentDeps, request: UserRequest, history: list, *, model: object
) -> list:
    """Run Market Agent on a user-forwarded request."""
    output_to_panel(PANEL, f"[user] {request.content}")
    return await run_agent_streamed(
        market_agent,
        f"Analyze this request: {request.content}",
        deps=deps,
        history=history,
        model=model,
        panel=PANEL,
    )


async def run_on_ticker(
    deps: AgentDeps, symbol: str, history: list, *, model: object
) -> list:
    """Run Market Agent on a meaningful ticker update."""
    ticker = deps.state.tickers.get(symbol)
    if not ticker:
        return history
    prompt = (
        f"Price update for {symbol}: "
        f"bid={ticker.bid} ask={ticker.ask} last={ticker.last} vol={ticker.volume}. "
        f"Assess if there's a trading opportunity."
    )
    return await run_agent_streamed(
        market_agent, prompt, deps=deps, history=history, model=model, panel=PANEL
    )


async def run_on_consultation(
    deps: AgentDeps, consult: ConsultMarket, history: list, *, model: object
) -> list:
    """Run Market Agent on a Risk Agent consultation."""
    output_to_panel(PANEL, f"[risk asks] {consult.symbol}: {consult.question}")
    return await run_agent_streamed(
        market_agent,
        f"Risk Agent asks about {consult.symbol}: {consult.question}. "
        f"Use respond_to_consultation tool to reply.",
        deps=deps,
        history=history,
        model=model,
        panel=PANEL,
    )
