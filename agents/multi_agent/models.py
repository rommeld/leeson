"""Pydantic models for inter-agent messages and shared dependencies."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING

from pydantic import BaseModel

from multi_agent.bridge import send_token_usage

if TYPE_CHECKING:
    from multi_agent.bus import AgentBus
    from multi_agent.state import SharedState


class AgentRole(str, Enum):
    """Identifies each agent for message routing."""

    USER = "user"
    MARKET = "market"
    IDEATION = "ideation"
    RISK = "risk"
    EXECUTION = "execution"


# -- Inter-agent messages --


class UserRequest(BaseModel):
    """Operator's analysis request forwarded from User Agent to Market Agent."""

    content: str


class TradeIdea(BaseModel):
    """Proposed trade from Market Agent to Risk Agent."""

    symbol: str
    side: str  # "buy" or "sell"
    reason: str
    probability: float  # 0.0 to 1.0 confidence score
    suggested_qty: str
    suggested_price: str | None = None
    order_type: str = "limit"


class ConsultMarket(BaseModel):
    """Risk Agent asking Market Agent about an existing position."""

    symbol: str
    question: str


class MarketAnalysis(BaseModel):
    """Market Agent's response to a Risk consultation."""

    symbol: str
    analysis: str
    recommendation: str  # "hold", "close", "add"


class ApprovedOrder(BaseModel):
    """Sized order approved by Risk Agent for Execution Agent."""

    symbol: str
    side: str
    order_type: str
    qty: str
    price: str | None = None
    cl_ord_id: str | None = None
    reason: str


class ClosePosition(BaseModel):
    """Risk Agent instructs Execution Agent to close a position."""

    symbol: str
    side: str  # opposite side to close
    qty: str
    reason: str


class OrderPlaced(BaseModel):
    """Execution Agent notifies Risk and Market of order result."""

    symbol: str
    side: str
    qty: str
    success: bool
    order_id: str | None = None
    error: str | None = None


class OrderFilled(BaseModel):
    """Execution update from exchange, forwarded to Risk Agent."""

    data: list[dict]


class TickerBroadcast(BaseModel):
    """Price update for an active pair, forwarded to Market Agent."""

    data: dict


# -- Union type for bus messages --


AgentMessage = (
    UserRequest
    | TradeIdea
    | ConsultMarket
    | MarketAnalysis
    | ApprovedOrder
    | ClosePosition
    | OrderPlaced
    | OrderFilled
    | TickerBroadcast
)


@dataclass
class AgentDeps:
    """Dependencies injected into all pydantic-ai agents."""

    state: SharedState
    bus: AgentBus
    output_panel: int


def record_usage(deps: AgentDeps, result: object) -> None:
    """Record token usage from an agent run result and report to the TUI."""
    usage = result.usage()
    deps.state.total_input_tokens += usage.request_tokens or 0
    deps.state.total_output_tokens += usage.response_tokens or 0
    send_token_usage(deps.state.total_input_tokens, deps.state.total_output_tokens)
