"""User Agent — operator interface (Panel 0).

Receives typed input from the TUI, interprets operator intent,
and forwards analysis requests to the Market Agent. Never makes
trading decisions directly.
"""

from __future__ import annotations

from pydantic_ai import Agent, RunContext

from multi_agent.bridge import output_to_panel
from multi_agent.models import AgentDeps, AgentRole, UserRequest, record_usage

PANEL = 0

user_agent = Agent(
    model=None,
    defer_model_check=True,
    deps_type=AgentDeps,
    system_prompt=(
        "You are the User Agent for the Leeson crypto trading system. "
        "You are the operator's interface — concise, professional, and helpful. "
        "Your role:\n"
        "- Interpret operator commands and questions\n"
        "- Forward analysis requests to the Market Agent\n"
        "- Report system status when asked\n"
        "- Update trading pair watchlists\n"
        "You NEVER make trading decisions or place orders. "
        "Keep responses brief — operators want information, not essays."
    ),
)


@user_agent.instructions
async def dynamic_context(ctx: RunContext[AgentDeps]) -> str:
    """Inject current system state into the prompt."""
    state = ctx.deps.state
    parts = []
    if state.risk_limits:
        parts.append(f"Risk limits: {state.risk_limits}")
    if state.active_pairs:
        parts.append(f"Active pairs: {', '.join(state.active_pairs)}")
    parts.append(f"Token state: {state.token_state}")
    if state.positions:
        pos_summary = ", ".join(
            f"{p.symbol} {p.side} {p.qty}" for p in state.positions.values()
        )
        parts.append(f"Open positions: {pos_summary}")
    return "\n".join(parts) if parts else "No active state yet."


@user_agent.tool
async def forward_to_market_agent(
    ctx: RunContext[AgentDeps], request: str
) -> str:
    """Forward an analysis request to the Market Agent.

    Args:
        request: The analysis question or request to send.
    """
    await ctx.deps.bus.send(AgentRole.MARKET, UserRequest(content=request))
    return f"Forwarded to Market Agent: {request}"


@user_agent.tool
async def update_trading_pairs(
    ctx: RunContext[AgentDeps], pairs: list[str]
) -> str:
    """Update the list of actively traded pairs.

    Args:
        pairs: List of trading pair symbols (e.g. ["BTC/USD", "ETH/USD"]).
    """
    ctx.deps.state.active_pairs = pairs
    return f"Active pairs updated: {', '.join(pairs)}"


@user_agent.tool
async def get_system_status(ctx: RunContext[AgentDeps]) -> str:
    """Get current system status including positions, balances, and token state."""
    state = ctx.deps.state
    lines = [f"Token state: {state.token_state}"]
    if state.active_pairs:
        lines.append(f"Active pairs: {', '.join(state.active_pairs)}")
    if state.positions:
        for p in state.positions.values():
            lines.append(
                f"Position: {p.symbol} {p.side} qty={p.qty} "
                f"entry={p.entry_price} pnl={p.unrealized_pnl}"
            )
    else:
        lines.append("No open positions.")
    if state.balances:
        for b in state.balances.values():
            lines.append(f"Balance: {b.asset} = {b.balance}")
    if state.risk_limits:
        lines.append(f"Risk limits: {state.risk_limits}")
    return "\n".join(lines)


async def run_once(
    deps: AgentDeps, user_input: str, history: list, *, model: object
) -> list:
    """Run the user agent on a single user message, returning updated history."""
    output_to_panel(PANEL, f"> {user_input}")
    result = await user_agent.run(
        user_input, deps=deps, message_history=history, model=model
    )
    record_usage(deps, result)
    # Truncate history at 30 messages
    history = result.all_messages()[-30:]
    output_to_panel(PANEL, result.output)
    return history
