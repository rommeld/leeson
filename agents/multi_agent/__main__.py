"""Entrypoint for the multi-agent trading system.

Spawned by the Rust TUI as a subprocess. Communicates via JSON-lines
on stdin/stdout.

Usage: python -m multi_agent
"""

from __future__ import annotations

import asyncio
import sys

import logfire

from multi_agent import orchestrator

logfire.configure(
    service_name='leeson-agents',
    send_to_logfire='if-token-present',
    console=False,
)
logfire.instrument_pydantic_ai()
logfire.instrument_httpx()


def main() -> None:
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        loop.run_until_complete(orchestrator.run(loop))
    except KeyboardInterrupt:
        pass
    finally:
        loop.close()
        logfire.shutdown()


if __name__ == "__main__":
    main()
