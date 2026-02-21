"""Base class for Leeson trading agents.

Communicates with the Rust TUI over JSON-lines on stdin/stdout.
Uses only the standard library — no third-party dependencies.
"""

import json
import sys


class Agent:
    """Base agent that bridges stdin/stdout JSON-lines with the TUI.

    Subclasses override ``on_message`` and optionally ``on_shutdown``
    to implement agent logic. Call ``output()`` to write lines to a
    TUI panel.
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

    # -- Inbound message handlers (override in subclasses) --

    def on_message(self, content: str) -> None:
        """Called when the user sends a message from the TUI."""

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
