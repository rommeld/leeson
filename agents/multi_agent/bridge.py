"""Sync stdin/stdout bridge for async agent system.

A background thread reads JSON-lines from stdin (blocking) and pushes
parsed dicts into an asyncio queue. Writing to stdout is synchronous
via the GIL but wrapped for convenience.
"""

from __future__ import annotations

import asyncio
import json
import sys
import threading
from typing import Any


class StdinBridge:
    """Reads JSON-lines from stdin in a background thread, feeds an async queue."""

    def __init__(self, loop: asyncio.AbstractEventLoop) -> None:
        self._loop = loop
        self._queue: asyncio.Queue[dict | None] = asyncio.Queue()
        self._thread = threading.Thread(target=self._reader, daemon=True)

    def start(self) -> None:
        self._thread.start()

    async def recv(self) -> dict | None:
        """Wait for the next parsed JSON dict, or None on EOF/shutdown."""
        return await self._queue.get()

    def _reader(self) -> None:
        """Blocking stdin reader running in a background thread."""
        try:
            for raw in sys.stdin:
                raw = raw.strip()
                if not raw:
                    continue
                try:
                    msg = json.loads(raw)
                except json.JSONDecodeError:
                    continue
                asyncio.run_coroutine_threadsafe(
                    self._queue.put(msg), self._loop
                )
        except (KeyboardInterrupt, EOFError):
            pass
        # Signal EOF
        asyncio.run_coroutine_threadsafe(self._queue.put(None), self._loop)


def send_to_tui(obj: dict[str, Any]) -> None:
    """Write a JSON-lines message to stdout for the Rust TUI."""
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


def output_to_panel(panel: int, line: str) -> None:
    """Convenience: write a line to a specific TUI panel."""
    send_to_tui({"type": "output", "agent": panel, "line": line})


def send_ready() -> None:
    """Signal to the TUI that the agent subprocess is ready."""
    send_to_tui({"type": "ready"})


def send_error(message: str) -> None:
    """Report an error to the TUI."""
    send_to_tui({"type": "error", "message": message})


def send_place_order(
    symbol: str,
    side: str,
    order_type: str,
    qty: str,
    price: str | None = None,
    cl_ord_id: str | None = None,
) -> None:
    """Submit an order request to the TUI."""
    msg: dict[str, Any] = {
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
    send_to_tui(msg)
