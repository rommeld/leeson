"""Shared mutable state accessible by all agents."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class TickerSnapshot:
    """Latest price data for a trading pair."""

    symbol: str
    bid: str
    ask: str
    last: str
    volume: str
    raw: dict  # full ticker payload from exchange


@dataclass
class Position:
    """Tracked open position."""

    symbol: str
    side: str
    qty: str
    entry_price: str
    current_price: str = ""
    unrealized_pnl: str = ""


@dataclass
class BalanceInfo:
    """Account balance for a single asset."""

    asset: str
    balance: str


@dataclass
class SharedState:
    """Mutable state shared across all agents.

    Accessed within a single asyncio event loop, so no locking needed.
    """

    tickers: dict[str, TickerSnapshot] = field(default_factory=dict)
    positions: dict[str, Position] = field(default_factory=dict)
    balances: dict[str, BalanceInfo] = field(default_factory=dict)
    risk_limits: str = ""
    active_pairs: list[str] = field(default_factory=list)
    token_state: str = "unknown"
    shutting_down: bool = False
    # Track last analyzed price per symbol for rate limiting
    last_analyzed_price: dict[str, float] = field(default_factory=dict)
