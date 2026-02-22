"""Risk Agent — conservative gatekeeper (Panel 2, prefixed [risk]).

Evaluates trade ideas, enforces risk limits, monitors open positions,
and sends approved orders to the Execution Agent. Never exceeds limits.
Cuts losses at 2% of position value.
"""

from __future__ import annotations

from pydantic_ai import Agent, RunContext

from multi_agent.bridge import output_to_panel
from multi_agent.models import (
    AgentDeps,
    AgentRole,
    ApprovedOrder,
    ClosePosition,
    ConsultMarket,
    MarketAnalysis,
    TradeIdea,
    record_usage,
)

PANEL = 2

risk_agent = Agent(
    model=None,
    defer_model_check=True,
    deps_type=AgentDeps,
    system_prompt=(
        "You are the Risk Agent for the Leeson crypto trading system. "
        "You are a conservative gatekeeper — your job is to protect capital.\n\n"
        "Your rules:\n"
        "- NEVER exceed the configured risk limits\n"
        "- Cut losses at 2% of position value — no exceptions\n"
        "- Size positions conservatively relative to account balance\n"
        "- Require probability >= 0.6 to approve a trade idea\n"
        "- Consult Market Agent when uncertain about a position\n"
        "- Monitor all open positions and close losers proactively\n\n"
        "You approve or reject trade ideas, and you can close positions. "
        "Always explain your reasoning briefly. Prefix all output with [risk]."
    ),
)


@risk_agent.instructions
async def dynamic_context(ctx: RunContext[AgentDeps]) -> str:
    """Inject risk limits, positions, and token state."""
    state = ctx.deps.state
    parts = []
    if state.risk_limits:
        parts.append(f"Risk limits: {state.risk_limits}")
    parts.append(f"Token state: {state.token_state}")
    if state.positions:
        parts.append("Open positions:")
        for p in state.positions.values():
            parts.append(
                f"  {p.symbol} {p.side} qty={p.qty} "
                f"entry={p.entry_price} current={p.current_price} "
                f"pnl={p.unrealized_pnl}"
            )
    else:
        parts.append("No open positions.")
    if state.balances:
        for b in state.balances.values():
            parts.append(f"Balance: {b.asset} = {b.balance}")
    return "\n".join(parts)


@risk_agent.tool
async def approve_order(
    ctx: RunContext[AgentDeps],
    symbol: str,
    side: str,
    order_type: str,
    qty: str,
    reason: str,
    price: str | None = None,
    cl_ord_id: str | None = None,
) -> str:
    """Approve and send an order to the Execution Agent.

    Args:
        symbol: Trading pair.
        side: "buy" or "sell".
        order_type: "limit" or "market".
        qty: Order quantity.
        reason: Why this order is approved.
        price: Limit price (required for limit orders).
        cl_ord_id: Optional client order ID.
    """
    if ctx.deps.state.token_state != "authenticated":
        return "Cannot place orders — not authenticated with exchange."
    order = ApprovedOrder(
        symbol=symbol,
        side=side,
        order_type=order_type,
        qty=qty,
        price=price,
        cl_ord_id=cl_ord_id,
        reason=reason,
    )
    await ctx.deps.bus.send(AgentRole.EXECUTION, order)
    output_to_panel(
        PANEL, f"[risk] APPROVED: {symbol} {side} {qty} — {reason}"
    )
    return f"Order approved and sent to Execution: {symbol} {side} {qty}"


@risk_agent.tool
async def reject_trade_idea(
    ctx: RunContext[AgentDeps], symbol: str, side: str, reason: str
) -> str:
    """Reject a trade idea from the Market Agent.

    Args:
        symbol: The trading pair that was proposed.
        side: The proposed side.
        reason: Why the idea was rejected.
    """
    output_to_panel(
        PANEL, f"[risk] REJECTED: {symbol} {side} — {reason}"
    )
    return f"Trade idea rejected: {symbol} {side} — {reason}"


@risk_agent.tool
async def consult_market_agent(
    ctx: RunContext[AgentDeps], symbol: str, question: str
) -> str:
    """Ask the Market Agent about a position or market condition.

    Args:
        symbol: The trading pair to ask about.
        question: Specific question for the Market Agent.
    """
    await ctx.deps.bus.send(
        AgentRole.MARKET, ConsultMarket(symbol=symbol, question=question)
    )
    output_to_panel(
        PANEL, f"[risk] Consulting market on {symbol}: {question}"
    )
    return f"Consultation sent to Market Agent about {symbol}"


@risk_agent.tool
async def close_position(
    ctx: RunContext[AgentDeps],
    symbol: str,
    side: str,
    qty: str,
    reason: str,
) -> str:
    """Close an existing position by sending to Execution Agent.

    Args:
        symbol: Trading pair to close.
        side: Opposite side to current position (sell to close long, buy to close short).
        qty: Quantity to close.
        reason: Why the position is being closed.
    """
    close = ClosePosition(symbol=symbol, side=side, qty=qty, reason=reason)
    await ctx.deps.bus.send(AgentRole.EXECUTION, close)
    output_to_panel(
        PANEL, f"[risk] CLOSING: {symbol} {side} {qty} — {reason}"
    )
    return f"Close order sent to Execution: {symbol} {side} {qty}"


@risk_agent.tool
async def get_position_summary(ctx: RunContext[AgentDeps]) -> str:
    """Get a summary of all open positions and their PnL."""
    state = ctx.deps.state
    if not state.positions:
        return "No open positions."
    lines = []
    for p in state.positions.values():
        lines.append(
            f"{p.symbol} {p.side} qty={p.qty} "
            f"entry={p.entry_price} current={p.current_price} "
            f"pnl={p.unrealized_pnl}"
        )
    return "\n".join(lines)


async def run_on_trade_idea(
    deps: AgentDeps, idea: TradeIdea, history: list, *, model: object
) -> list:
    """Evaluate a trade idea from the Market Agent."""
    output_to_panel(
        PANEL,
        f"[risk] Evaluating: {idea.symbol} {idea.side} "
        f"qty={idea.suggested_qty} p={idea.probability:.0%}",
    )
    prompt = (
        f"Evaluate this trade idea:\n"
        f"Symbol: {idea.symbol}, Side: {idea.side}, Type: {idea.order_type}\n"
        f"Qty: {idea.suggested_qty}, Price: {idea.suggested_price}\n"
        f"Probability: {idea.probability:.0%}\n"
        f"Reason: {idea.reason}\n\n"
        f"Approve or reject based on risk limits and position sizing rules."
    )
    result = await risk_agent.run(
        prompt, deps=deps, message_history=history, model=model
    )
    record_usage(deps, result)
    history = result.all_messages()[-30:]
    return history


async def run_on_market_analysis(
    deps: AgentDeps, analysis: MarketAnalysis, history: list, *, model: object
) -> list:
    """Process a consultation response from Market Agent."""
    prompt = (
        f"Market Agent responds about {analysis.symbol}:\n"
        f"Analysis: {analysis.analysis}\n"
        f"Recommendation: {analysis.recommendation}\n\n"
        f"Decide what action to take on this position."
    )
    result = await risk_agent.run(
        prompt, deps=deps, message_history=history, model=model
    )
    record_usage(deps, result)
    history = result.all_messages()[-30:]
    return history


async def run_on_execution_update(
    deps: AgentDeps, data: list[dict], history: list, *, model: object
) -> list:
    """Process execution updates (order fills, status changes)."""
    prompt = f"Execution update received: {data}. Review position state."
    result = await risk_agent.run(
        prompt, deps=deps, message_history=history, model=model
    )
    record_usage(deps, result)
    history = result.all_messages()[-30:]
    return history


async def run_position_review(
    deps: AgentDeps, history: list, *, model: object
) -> list:
    """Periodic review of all open positions (every 30 seconds)."""
    if not deps.state.positions:
        return history
    output_to_panel(PANEL, "[risk] Periodic position review...")
    prompt = (
        "Review all open positions. Check if any need to be closed "
        "(2% loss rule) or if the Market Agent should be consulted."
    )
    result = await risk_agent.run(
        prompt, deps=deps, message_history=history, model=model
    )
    record_usage(deps, result)
    history = result.all_messages()[-30:]
    return history
