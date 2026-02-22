"""Execution Agent — order placement (Panel 2, prefixed [exec]).

Executes orders exactly as specified by the Risk Agent. Never modifies
order parameters. Stateless — no message history needed.
"""

from __future__ import annotations

from pydantic_ai import Agent, RunContext

from multi_agent.bridge import output_to_panel, send_place_order
from multi_agent.models import (
    AgentDeps,
    AgentRole,
    ApprovedOrder,
    ClosePosition,
    OrderPlaced,
    record_usage,
)

PANEL = 2

execution_agent = Agent(
    model=None,
    defer_model_check=True,
    deps_type=AgentDeps,
    system_prompt=(
        "You are the Execution Agent for the Leeson crypto trading system. "
        "Your ONLY job is to execute orders exactly as specified. "
        "NEVER modify order parameters — quantity, price, side, symbol must "
        "match exactly what was approved. Use the place_order tool to submit "
        "orders. Report results faithfully. Prefix output with [exec]."
    ),
)


@execution_agent.tool
async def place_order(
    ctx: RunContext[AgentDeps],
    symbol: str,
    side: str,
    order_type: str,
    qty: str,
    price: str | None = None,
    cl_ord_id: str | None = None,
) -> str:
    """Place an order via the Rust TUI's WebSocket connection.

    Args:
        symbol: Trading pair (e.g. "BTC/USD").
        side: "buy" or "sell".
        order_type: "limit" or "market".
        qty: Order quantity.
        price: Limit price (required for limit orders).
        cl_ord_id: Optional client order ID for tracking.
    """
    send_place_order(
        symbol=symbol,
        side=side,
        order_type=order_type,
        qty=qty,
        price=price,
        cl_ord_id=cl_ord_id,
    )
    output_to_panel(
        PANEL,
        f"[exec] ORDER SENT: {symbol} {side} {order_type} qty={qty} "
        f"price={price or 'market'}",
    )
    return f"Order submitted: {symbol} {side} {order_type} qty={qty}"


@execution_agent.tool
async def report_order_result(
    ctx: RunContext[AgentDeps],
    symbol: str,
    side: str,
    qty: str,
    success: bool,
    order_id: str | None = None,
    error: str | None = None,
) -> str:
    """Notify Risk and Market agents of an order result.

    Args:
        symbol: Trading pair.
        side: Order side.
        qty: Order quantity.
        success: Whether the order was accepted.
        order_id: Exchange order ID if successful.
        error: Error message if failed.
    """
    notification = OrderPlaced(
        symbol=symbol,
        side=side,
        qty=qty,
        success=success,
        order_id=order_id,
        error=error,
    )
    await ctx.deps.bus.send(AgentRole.RISK, notification)
    await ctx.deps.bus.send(AgentRole.MARKET, notification)
    status = "SUCCESS" if success else f"FAILED: {error}"
    output_to_panel(PANEL, f"[exec] Result: {symbol} {side} — {status}")
    return f"Order result reported: {status}"


async def run_on_approved_order(
    deps: AgentDeps, order: ApprovedOrder, *, model: object
) -> None:
    """Execute an approved order from Risk Agent."""
    output_to_panel(
        PANEL,
        f"[exec] Executing: {order.symbol} {order.side} {order.order_type} "
        f"qty={order.qty} price={order.price or 'market'}",
    )
    prompt = (
        f"Execute this approved order exactly as specified:\n"
        f"Symbol: {order.symbol}, Side: {order.side}, Type: {order.order_type}\n"
        f"Qty: {order.qty}, Price: {order.price}\n"
        f"Reason: {order.reason}\n\n"
        f"Use the place_order tool with these exact parameters."
    )
    result = await execution_agent.run(prompt, deps=deps, model=model)
    record_usage(deps, result)


async def run_on_close_position(
    deps: AgentDeps, close: ClosePosition, *, model: object
) -> None:
    """Execute a position close from Risk Agent."""
    output_to_panel(
        PANEL,
        f"[exec] Closing: {close.symbol} {close.side} qty={close.qty}",
    )
    prompt = (
        f"Close this position exactly as specified:\n"
        f"Symbol: {close.symbol}, Side: {close.side}, Qty: {close.qty}\n"
        f"Reason: {close.reason}\n\n"
        f"Use the place_order tool with order_type='market'."
    )
    result = await execution_agent.run(prompt, deps=deps, model=model)
    record_usage(deps, result)


async def run_on_order_response(
    deps: AgentDeps,
    success: bool,
    order_id: str | None,
    cl_ord_id: str | None,
    error: str | None,
) -> None:
    """Handle an order response from the exchange."""
    status = "accepted" if success else f"rejected: {error}"
    output_to_panel(
        PANEL, f"[exec] Exchange response: order_id={order_id} — {status}"
    )
    if not success:
        # Notify Risk Agent of failure
        notification = OrderPlaced(
            symbol="",
            side="",
            qty="",
            success=False,
            order_id=order_id,
            error=error,
        )
        await deps.bus.send(AgentRole.RISK, notification)
