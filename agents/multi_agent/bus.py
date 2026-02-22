"""Inter-agent async message bus."""

from __future__ import annotations

import asyncio

from multi_agent.models import AgentMessage, AgentRole


class AgentBus:
    """Routes messages between agents via per-role async queues."""

    def __init__(self) -> None:
        self._queues: dict[AgentRole, asyncio.Queue[AgentMessage]] = {
            role: asyncio.Queue() for role in AgentRole
        }

    async def send(self, to: AgentRole, message: AgentMessage) -> None:
        """Send a message to a specific agent's queue."""
        await self._queues[to].put(message)

    async def recv(self, role: AgentRole) -> AgentMessage:
        """Wait for the next message in an agent's queue."""
        return await self._queues[role].get()

    async def broadcast(
        self, message: AgentMessage, *, exclude: AgentRole | None = None
    ) -> None:
        """Send a message to all agents except the excluded one."""
        for role, queue in self._queues.items():
            if role != exclude:
                await queue.put(message)
