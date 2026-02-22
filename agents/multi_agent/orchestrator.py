"""Main event loop — routes messages between the TUI bridge and agents.

Runs 6 concurrent asyncio tasks:
1. route_stdin_messages — stdin bridge → agent bus
2. run_user_agent_loop — process User Agent queue
3. run_market_agent_loop — process Market Agent queue
4. run_risk_agent_loop — process Risk Agent queue
5. run_execution_agent_loop — process Execution Agent queue
6. run_risk_monitor — periodic position review (every 30s)
"""

from __future__ import annotations

import asyncio
import sys
import traceback

from multi_agent.bridge import StdinBridge, output_to_panel, send_ready
from multi_agent.bus import AgentBus
from multi_agent.llm import create_model
from multi_agent.models import (
    AgentDeps,
    AgentRole,
    ApprovedOrder,
    ClosePosition,
    ConsultMarket,
    MarketAnalysis,
    OrderFilled,
    OrderPlaced,
    TickerBroadcast,
    TradeIdea,
    UserRequest,
)
from multi_agent.state import BalanceInfo, SharedState, TickerSnapshot

from multi_agent.agents import (
    execution_agent,
    market_agent,
    risk_agent,
    user_agent,
)

# Minimum price change (fraction) to forward ticker to Market Agent
TICKER_CHANGE_THRESHOLD = 0.001  # 0.1%

RISK_MONITOR_INTERVAL = 30  # seconds


async def run(loop: asyncio.AbstractEventLoop) -> None:
    """Start the multi-agent system."""
    state = SharedState()
    bus = AgentBus()
    bridge = StdinBridge(loop)
    bridge.start()

    # Create shared LLM model instance (deferred until runtime so
    # FIREWORKS_API_KEY is available)
    model = create_model()

    # Create per-agent deps with their output panel assignments
    user_deps = AgentDeps(state=state, bus=bus, output_panel=0)
    market_deps = AgentDeps(state=state, bus=bus, output_panel=1)
    risk_deps = AgentDeps(state=state, bus=bus, output_panel=2)
    exec_deps = AgentDeps(state=state, bus=bus, output_panel=2)

    send_ready()

    tasks = [
        asyncio.create_task(
            _route_stdin_messages(bridge, bus, state),
            name="route_stdin",
        ),
        asyncio.create_task(
            _run_user_agent_loop(bus, user_deps, model),
            name="user_agent",
        ),
        asyncio.create_task(
            _run_market_agent_loop(bus, market_deps, model),
            name="market_agent",
        ),
        asyncio.create_task(
            _run_risk_agent_loop(bus, risk_deps, model),
            name="risk_agent",
        ),
        asyncio.create_task(
            _run_execution_agent_loop(bus, exec_deps, model),
            name="execution_agent",
        ),
        asyncio.create_task(
            _run_risk_monitor(risk_deps, model),
            name="risk_monitor",
        ),
    ]

    try:
        # Wait until any task completes (usually route_stdin on shutdown)
        done, pending = await asyncio.wait(
            tasks, return_when=asyncio.FIRST_COMPLETED
        )
        # Check for unexpected errors
        for task in done:
            if task.exception():
                output_to_panel(
                    0,
                    f"[error] Task {task.get_name()} failed: {task.exception()}",
                )
    finally:
        state.shutting_down = True
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)


async def _route_stdin_messages(
    bridge: StdinBridge, bus: AgentBus, state: SharedState
) -> None:
    """Read from stdin bridge and dispatch to appropriate agents."""
    while not state.shutting_down:
        msg = await bridge.recv()
        if msg is None:
            break

        msg_type = msg.get("type")
        try:
            if msg_type == "user_message":
                content = msg.get("content", "")
                await bus.send(AgentRole.USER, UserRequest(content=content))

            elif msg_type == "ticker_update":
                data = msg.get("data", {})
                symbol = data.get("symbol", "")
                _update_ticker(state, symbol, data)
                # Only forward to Market if active pair with meaningful change
                if symbol in state.active_pairs and _price_changed_enough(
                    state, symbol, data
                ):
                    await bus.send(
                        AgentRole.MARKET, TickerBroadcast(data=data)
                    )

            elif msg_type == "execution_update":
                data = msg.get("data", [])
                await bus.send(AgentRole.RISK, OrderFilled(data=data))

            elif msg_type == "order_response":
                await bus.send(
                    AgentRole.EXECUTION,
                    OrderPlaced(
                        symbol="",
                        side="",
                        qty="",
                        success=msg.get("success", False),
                        order_id=msg.get("order_id"),
                        error=msg.get("error"),
                    ),
                )

            elif msg_type == "balance_update":
                for item in msg.get("data", []):
                    asset = item.get("asset", item.get("name", ""))
                    balance = item.get("balance", item.get("amount", ""))
                    state.balances[asset] = BalanceInfo(
                        asset=asset, balance=str(balance)
                    )

            elif msg_type == "active_pairs":
                state.active_pairs = msg.get("pairs", [])
                output_to_panel(
                    0,
                    f"[system] Active pairs: {', '.join(state.active_pairs) or 'none'}",
                )

            elif msg_type == "risk_limits":
                state.risk_limits = msg.get("description", "")
                output_to_panel(2, f"[risk] Limits updated: {state.risk_limits}")

            elif msg_type == "token_state":
                state.token_state = msg.get("state", "unknown")
                output_to_panel(0, f"Token state: {state.token_state}")

            elif msg_type == "shutdown":
                output_to_panel(0, "[system] Shutdown requested")
                state.shutting_down = True
                break

        except Exception:
            traceback.print_exc(file=sys.stderr)


def _update_ticker(state: SharedState, symbol: str, data: dict) -> None:
    """Update the shared ticker state from raw data."""
    state.tickers[symbol] = TickerSnapshot(
        symbol=symbol,
        bid=str(data.get("bid", "")),
        ask=str(data.get("ask", "")),
        last=str(data.get("last", "")),
        volume=str(data.get("volume", "")),
        raw=data,
    )
    # Update current price on any matching position
    for pos in state.positions.values():
        if pos.symbol == symbol:
            pos.current_price = str(data.get("last", ""))


def _price_changed_enough(
    state: SharedState, symbol: str, data: dict
) -> bool:
    """Check if price changed more than TICKER_CHANGE_THRESHOLD from last analysis."""
    try:
        current = float(data.get("last", 0))
    except (ValueError, TypeError):
        return False
    if current == 0:
        return False
    last = state.last_analyzed_price.get(symbol, 0.0)
    if last == 0:
        state.last_analyzed_price[symbol] = current
        return True
    change = abs(current - last) / last
    if change >= TICKER_CHANGE_THRESHOLD:
        state.last_analyzed_price[symbol] = current
        return True
    return False


async def _run_user_agent_loop(
    bus: AgentBus, deps: AgentDeps, model: object
) -> None:
    """Process User Agent message queue."""
    history: list = []
    while not deps.state.shutting_down:
        msg = await bus.recv(AgentRole.USER)
        if deps.state.shutting_down:
            break
        try:
            if isinstance(msg, UserRequest):
                history = await user_agent.run_once(
                    deps, msg.content, history, model=model
                )
        except Exception:
            traceback.print_exc(file=sys.stderr)
            output_to_panel(0, "[error] User Agent encountered an error")


async def _run_market_agent_loop(
    bus: AgentBus, deps: AgentDeps, model: object
) -> None:
    """Process Market Agent message queue."""
    history: list = []
    while not deps.state.shutting_down:
        msg = await bus.recv(AgentRole.MARKET)
        if deps.state.shutting_down:
            break
        try:
            if isinstance(msg, UserRequest):
                history = await market_agent.run_on_user_request(
                    deps, msg, history, model=model
                )
            elif isinstance(msg, TickerBroadcast):
                symbol = msg.data.get("symbol", "")
                history = await market_agent.run_on_ticker(
                    deps, symbol, history, model=model
                )
            elif isinstance(msg, ConsultMarket):
                history = await market_agent.run_on_consultation(
                    deps, msg, history, model=model
                )
            elif isinstance(msg, OrderPlaced):
                # Informational — no LLM call needed
                status = "filled" if msg.success else f"failed: {msg.error}"
                output_to_panel(
                    1, f"[order] {msg.symbol} {msg.side} — {status}"
                )
        except Exception:
            traceback.print_exc(file=sys.stderr)
            output_to_panel(1, "[error] Market Agent encountered an error")


async def _run_risk_agent_loop(
    bus: AgentBus, deps: AgentDeps, model: object
) -> None:
    """Process Risk Agent message queue."""
    history: list = []
    while not deps.state.shutting_down:
        msg = await bus.recv(AgentRole.RISK)
        if deps.state.shutting_down:
            break
        try:
            if isinstance(msg, TradeIdea):
                history = await risk_agent.run_on_trade_idea(
                    deps, msg, history, model=model
                )
            elif isinstance(msg, MarketAnalysis):
                history = await risk_agent.run_on_market_analysis(
                    deps, msg, history, model=model
                )
            elif isinstance(msg, OrderFilled):
                history = await risk_agent.run_on_execution_update(
                    deps, msg.data, history, model=model
                )
            elif isinstance(msg, OrderPlaced):
                if not msg.success:
                    output_to_panel(
                        2,
                        f"[risk] Order failed: {msg.error}",
                    )
        except Exception:
            traceback.print_exc(file=sys.stderr)
            output_to_panel(2, "[risk] [error] Risk Agent encountered an error")


async def _run_execution_agent_loop(
    bus: AgentBus, deps: AgentDeps, model: object
) -> None:
    """Process Execution Agent message queue."""
    while not deps.state.shutting_down:
        msg = await bus.recv(AgentRole.EXECUTION)
        if deps.state.shutting_down:
            break
        try:
            if isinstance(msg, ApprovedOrder):
                await execution_agent.run_on_approved_order(
                    deps, msg, model=model
                )
            elif isinstance(msg, ClosePosition):
                await execution_agent.run_on_close_position(
                    deps, msg, model=model
                )
            elif isinstance(msg, OrderPlaced):
                # Order response from exchange
                await execution_agent.run_on_order_response(
                    deps,
                    success=msg.success,
                    order_id=msg.order_id,
                    cl_ord_id=None,
                    error=msg.error,
                )
        except Exception:
            traceback.print_exc(file=sys.stderr)
            output_to_panel(2, "[exec] [error] Execution Agent encountered an error")


async def _run_risk_monitor(deps: AgentDeps, model: object) -> None:
    """Periodic position review by Risk Agent."""
    history: list = []
    while not deps.state.shutting_down:
        await asyncio.sleep(RISK_MONITOR_INTERVAL)
        if deps.state.shutting_down:
            break
        try:
            history = await risk_agent.run_position_review(
                deps, history, model=model
            )
        except Exception:
            traceback.print_exc(file=sys.stderr)
