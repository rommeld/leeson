"""Base class for Leeson trading agents.

Communicates with the Rust TUI over JSON-lines on stdin/stdout.
Uses only the standard library — no third-party dependencies.
"""

import json
import sys


class Agent:
    """Base agent that bridges stdin/stdout JSON-lines with the TUI.

    Subclasses override ``on_message`` and optionally other ``on_*``
    callbacks to implement agent logic. Call ``output()`` to write
    lines to a TUI panel, and ``place_order()`` to submit orders.
    """

    def __init__(self, agent_index: int) -> None:
        self.agent_index = agent_index

    # -- Outbound messages (agent → TUI) --

    def output(self, line: str, panel: int | None = None) -> None:
        """Write a line to an agent output panel in the TUI."""
        target = panel if panel is not None else self.agent_index
        self._send({"type": "output", "agent": target, "line": line})

    def error(self, message: str) -> None:
        """Report an error to the TUI."""
        self._send({"type": "error", "message": message})

    def ready(self) -> None:
        """Signal that the agent is ready to receive messages."""
        self._send({"type": "ready"})

    def place_order(
        self,
        symbol: str,
        side: str,
        order_type: str,
        qty: str,
        price: str | None = None,
        cl_ord_id: str | None = None,
    ) -> None:
        """Submit an order request to the TUI for risk check and execution."""
        msg: dict = {
            "type": "place_order",
            "symbol": symbol,
            "side": side,
            "order_type": order_type,
            "qty": qty,
        }
        if price is not None:
            msg["price"] = price
        if cl_ord_id is not None:
            msg["cl_ord_id"] = cl_ord_id
        self._send(msg)

    # -- Inbound message handlers (override in subclasses) --

    def on_message(self, content: str) -> None:
        """Called when the user sends a message from the TUI."""

    def on_execution(self, data: list[dict]) -> None:
        """Called on order status changes and trade execution events."""

    def on_ticker(self, data: dict) -> None:
        """Called on throttled price snapshots for a trading pair."""

    def on_trade(self, data: list[dict]) -> None:
        """Called on market trades."""

    def on_balance(self, data: list[dict]) -> None:
        """Called on balance changes."""

    def on_order_response(
        self,
        success: bool,
        order_id: str | None,
        cl_ord_id: str | None,
        order_userref: int | None,
        error: str | None,
    ) -> None:
        """Called with the structured result of an order placement."""

    def on_risk_limits(self, description: str) -> None:
        """Called when risk configuration is sent to the agent."""

    def on_token_state(self, state: str) -> None:
        """Called when the authentication token state changes."""

    def on_shutdown(self) -> None:
        """Called when the TUI requests a graceful shutdown."""

    # -- Main loop --

    def run(self) -> None:
        """Read JSON-lines from stdin and dispatch to handlers."""
        self.ready()
        try:
            for raw in sys.stdin:
                raw = raw.strip()
                if not raw:
                    continue
                try:
                    msg = json.loads(raw)
                except json.JSONDecodeError:
                    continue
                msg_type = msg.get("type")
                if msg_type == "user_message":
                    self.on_message(msg.get("content", ""))
                elif msg_type == "execution_update":
                    self.on_execution(msg.get("data", []))
                elif msg_type == "ticker_update":
                    self.on_ticker(msg.get("data", {}))
                elif msg_type == "trade_update":
                    self.on_trade(msg.get("data", []))
                elif msg_type == "balance_update":
                    self.on_balance(msg.get("data", []))
                elif msg_type == "order_response":
                    self.on_order_response(
                        success=msg.get("success", False),
                        order_id=msg.get("order_id"),
                        cl_ord_id=msg.get("cl_ord_id"),
                        order_userref=msg.get("order_userref"),
                        error=msg.get("error"),
                    )
                elif msg_type == "risk_limits":
                    self.on_risk_limits(msg.get("description", ""))
                elif msg_type == "token_state":
                    self.on_token_state(msg.get("state", ""))
                elif msg_type == "shutdown":
                    self.on_shutdown()
                    break
        except KeyboardInterrupt:
            pass

    # -- Internal --

    def _send(self, obj: dict) -> None:
        """Write a JSON object as a single line to stdout."""
        sys.stdout.write(json.dumps(obj) + "\n")
        sys.stdout.flush()
