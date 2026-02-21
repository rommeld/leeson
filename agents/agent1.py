#!/usr/bin/env python3
"""Echo agent for testing the TUI ↔ Python bridge.

Echoes user messages back to the Agent 1 output panel, proving the
round-trip works end-to-end.
"""

import sys

from leeson_agent import Agent


class EchoAgent(Agent):
    def on_message(self, content: str) -> None:
        self.output(f"Echo: {content}")

    def on_shutdown(self) -> None:
        self.output("[shutting down]")


if __name__ == "__main__":
    index = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    EchoAgent(index).run()
