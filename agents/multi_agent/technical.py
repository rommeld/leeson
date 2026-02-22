"""Technical analysis indicators — pure Python, no external dependencies.

All functions operate on plain lists. Designed for the ≤720-candle datasets
returned by the Kraken OHLC API.
"""

from __future__ import annotations

import math
from typing import NamedTuple


class Candle(NamedTuple):
    """Single OHLC candle as returned by Kraken."""

    time: int
    open: float
    high: float
    low: float
    close: float
    vwap: float
    volume: float
    count: int


def parse_candles(raw: list[list]) -> list[Candle]:
    """Convert Kraken's raw arrays into typed Candle tuples."""
    return [
        Candle(
            time=int(c[0]),
            open=float(c[1]),
            high=float(c[2]),
            low=float(c[3]),
            close=float(c[4]),
            vwap=float(c[5]),
            volume=float(c[6]),
            count=int(c[7]),
        )
        for c in raw
    ]


# ---------------------------------------------------------------------------
# Moving averages
# ---------------------------------------------------------------------------


def sma(values: list[float], period: int) -> list[float | None]:
    """Simple Moving Average."""
    result: list[float | None] = [None] * len(values)
    if period <= 0 or len(values) < period:
        return result
    window_sum = sum(values[:period])
    result[period - 1] = window_sum / period
    for i in range(period, len(values)):
        window_sum += values[i] - values[i - period]
        result[i] = window_sum / period
    return result


def ema(values: list[float], period: int) -> list[float | None]:
    """Exponential Moving Average."""
    result: list[float | None] = [None] * len(values)
    if period <= 0 or len(values) < period:
        return result
    # Seed with SMA of first `period` values
    seed = sum(values[:period]) / period
    result[period - 1] = seed
    k = 2.0 / (period + 1)
    prev = seed
    for i in range(period, len(values)):
        val = values[i] * k + prev * (1 - k)
        result[i] = val
        prev = val
    return result


# ---------------------------------------------------------------------------
# Oscillators
# ---------------------------------------------------------------------------


def rsi(closes: list[float], period: int = 14) -> list[float | None]:
    """Relative Strength Index using Wilder's smoothing."""
    result: list[float | None] = [None] * len(closes)
    if period <= 0 or len(closes) < period + 1:
        return result

    gains: list[float] = []
    losses: list[float] = []
    for i in range(1, len(closes)):
        delta = closes[i] - closes[i - 1]
        gains.append(max(delta, 0.0))
        losses.append(max(-delta, 0.0))

    # Initial averages over first `period` changes
    avg_gain = sum(gains[:period]) / period
    avg_loss = sum(losses[:period]) / period

    if avg_loss == 0:
        result[period] = 100.0
    else:
        rs = avg_gain / avg_loss
        result[period] = 100.0 - 100.0 / (1.0 + rs)

    for i in range(period, len(gains)):
        avg_gain = (avg_gain * (period - 1) + gains[i]) / period
        avg_loss = (avg_loss * (period - 1) + losses[i]) / period
        if avg_loss == 0:
            result[i + 1] = 100.0
        else:
            rs = avg_gain / avg_loss
            result[i + 1] = 100.0 - 100.0 / (1.0 + rs)

    return result


def macd(
    closes: list[float],
    fast: int = 12,
    slow: int = 26,
    signal_period: int = 9,
) -> tuple[list[float | None], list[float | None], list[float | None]]:
    """MACD line, signal line, and histogram."""
    fast_ema = ema(closes, fast)
    slow_ema = ema(closes, slow)

    n = len(closes)
    macd_line: list[float | None] = [None] * n
    for i in range(n):
        if fast_ema[i] is not None and slow_ema[i] is not None:
            macd_line[i] = fast_ema[i] - slow_ema[i]

    # Compute signal as EMA of non-None MACD values, mapped back
    macd_values = [(i, v) for i, v in enumerate(macd_line) if v is not None]
    signal_line: list[float | None] = [None] * n
    histogram: list[float | None] = [None] * n

    if len(macd_values) >= signal_period:
        raw = [v for _, v in macd_values]
        sig = ema(raw, signal_period)
        for j, (orig_idx, _) in enumerate(macd_values):
            if sig[j] is not None:
                signal_line[orig_idx] = sig[j]
                histogram[orig_idx] = macd_line[orig_idx] - sig[j]  # type: ignore[operator]

    return macd_line, signal_line, histogram


# ---------------------------------------------------------------------------
# Volatility
# ---------------------------------------------------------------------------


def bollinger_bands(
    closes: list[float], period: int = 20, num_std: float = 2.0
) -> tuple[list[float | None], list[float | None], list[float | None]]:
    """Bollinger Bands — returns (upper, middle, lower)."""
    mid = sma(closes, period)
    n = len(closes)
    upper: list[float | None] = [None] * n
    lower: list[float | None] = [None] * n

    for i in range(period - 1, n):
        m = mid[i]
        if m is None:
            continue
        window = closes[i - period + 1 : i + 1]
        variance = sum((x - m) ** 2 for x in window) / period
        std = math.sqrt(variance)
        upper[i] = m + num_std * std
        lower[i] = m - num_std * std

    return upper, mid, lower


def atr(candles: list[Candle], period: int = 14) -> list[float | None]:
    """Average True Range using Wilder's smoothing."""
    n = len(candles)
    result: list[float | None] = [None] * n
    if period <= 0 or n < period + 1:
        return result

    true_ranges: list[float] = [candles[0].high - candles[0].low]
    for i in range(1, n):
        c = candles[i]
        prev_close = candles[i - 1].close
        tr = max(c.high - c.low, abs(c.high - prev_close), abs(c.low - prev_close))
        true_ranges.append(tr)

    # Initial ATR is SMA of first `period` TRs
    avg = sum(true_ranges[:period]) / period
    result[period - 1] = avg
    for i in range(period, n):
        avg = (avg * (period - 1) + true_ranges[i]) / period
        result[i] = avg

    return result


# ---------------------------------------------------------------------------
# Volume & momentum
# ---------------------------------------------------------------------------


def volume_sma(volumes: list[float], period: int = 20) -> list[float | None]:
    """Simple moving average of volume."""
    return sma(volumes, period)


def price_momentum(closes: list[float], period: int = 10) -> list[float | None]:
    """Percentage price change over N candles."""
    result: list[float | None] = [None] * len(closes)
    for i in range(period, len(closes)):
        if closes[i - period] != 0:
            result[i] = (closes[i] - closes[i - period]) / closes[i - period] * 100.0
    return result


# ---------------------------------------------------------------------------
# Key levels
# ---------------------------------------------------------------------------


def find_key_levels(
    candles: list[Candle],
    lookback: int = 5,
    tolerance_pct: float = 0.5,
) -> dict[str, list[float]]:
    """Detect swing highs/lows and cluster them into support/resistance.

    Returns ``{"support": [...], "resistance": [...]}`` sorted by proximity
    to the current price, at most 3 of each.
    """
    if len(candles) < 2 * lookback + 1:
        return {"support": [], "resistance": []}

    swing_highs: list[float] = []
    swing_lows: list[float] = []

    for i in range(lookback, len(candles) - lookback):
        high = candles[i].high
        low = candles[i].low
        if all(high >= candles[j].high for j in range(i - lookback, i + lookback + 1) if j != i):
            swing_highs.append(high)
        if all(low <= candles[j].low for j in range(i - lookback, i + lookback + 1) if j != i):
            swing_lows.append(low)

    current_price = candles[-1].close

    def cluster(levels: list[float]) -> list[float]:
        if not levels:
            return []
        levels_sorted = sorted(levels)
        clusters: list[list[float]] = [[levels_sorted[0]]]
        for lv in levels_sorted[1:]:
            if (lv - clusters[-1][0]) / clusters[-1][0] * 100 <= tolerance_pct:
                clusters[-1].append(lv)
            else:
                clusters.append([lv])
        averaged = [sum(c) / len(c) for c in clusters]
        averaged.sort(key=lambda x: abs(x - current_price))
        return averaged[:3]

    return {
        "support": cluster(swing_lows),
        "resistance": cluster(swing_highs),
    }


# ---------------------------------------------------------------------------
# Composite formatter
# ---------------------------------------------------------------------------


def _rsi_label(value: float) -> str:
    if value >= 70:
        return "overbought"
    if value >= 60:
        return "neutral-bullish"
    if value >= 40:
        return "neutral"
    if value >= 30:
        return "neutral-bearish"
    return "oversold"


def _macd_label(hist: float | None, prev_hist: float | None) -> str:
    if hist is None:
        return "n/a"
    direction = "bullish" if hist > 0 else "bearish"
    if prev_hist is not None:
        expanding = abs(hist) > abs(prev_hist)
        direction += ", expanding" if expanding else ", contracting"
    return direction


def _fmt(value: float | None, decimals: int = 1) -> str:
    if value is None:
        return "n/a"
    if abs(value) >= 1000:
        return f"{value:,.{decimals}f}"
    return f"{value:.{decimals}f}"


def _fmt_signed(value: float | None, decimals: int = 1) -> str:
    if value is None:
        return "n/a"
    if abs(value) >= 1000:
        return f"{value:+,.{decimals}f}"
    return f"{value:+.{decimals}f}"


def compute_all(candles: list[Candle], interval: int = 60) -> str:
    """Run all indicators and return a compact text summary (~300 tokens)."""
    if len(candles) < 2:
        return "Insufficient data for technical analysis."

    closes = [c.close for c in candles]
    volumes = [c.volume for c in candles]
    current = closes[-1]

    # Trend
    ema9 = ema(closes, 9)
    ema21 = ema(closes, 21)
    mom = price_momentum(closes, 10)

    # RSI
    rsi_vals = rsi(closes, 14)

    # MACD
    macd_line, signal_line, hist = macd(closes)

    # Bollinger
    bb_upper, bb_mid, bb_lower = bollinger_bands(closes)

    # ATR
    atr_vals = atr(candles, 14)

    # Volume
    vol_avg = volume_sma(volumes, 20)

    # Key levels
    levels = find_key_levels(candles)

    # --- Build output ---
    lines: list[str] = [f"=== Technical Indicators ({interval}min) ==="]

    # Trend
    lines.append(
        f"TREND: EMA(9): {_fmt(ema9[-1])}  EMA(21): {_fmt(ema21[-1])}  "
        f"Momentum(10): {_fmt_signed(mom[-1])}%"
    )

    # RSI
    rsi_val = rsi_vals[-1]
    rsi_str = f"{_fmt(rsi_val)} ({_rsi_label(rsi_val)})" if rsi_val is not None else "n/a"
    lines.append(f"RSI(14): {rsi_str}")

    # MACD
    hist_label = _macd_label(hist[-1], hist[-2] if len(hist) >= 2 else None)
    lines.append(
        f"MACD: line={_fmt_signed(macd_line[-1])}  signal={_fmt_signed(signal_line[-1])}  "
        f"histogram={_fmt_signed(hist[-1])} ({hist_label})"
    )

    # Bollinger
    bb_pos = "n/a"
    if bb_upper[-1] is not None and bb_lower[-1] is not None:
        width = bb_upper[-1] - bb_lower[-1]
        if width > 0:
            bb_pos = f"{(current - bb_lower[-1]) / width * 100:.0f}%"
    lines.append(
        f"BOLLINGER(20,2): upper={_fmt(bb_upper[-1])}  mid={_fmt(bb_mid[-1])}  "
        f"lower={_fmt(bb_lower[-1])}  position={bb_pos}"
    )

    # ATR
    atr_val = atr_vals[-1]
    atr_pct = f"{atr_val / current * 100:.2f}% of price" if atr_val and current else "n/a"
    lines.append(f"ATR(14): {_fmt(atr_val)} ({atr_pct})")

    # Volume
    cur_vol = volumes[-1]
    avg_vol = vol_avg[-1]
    rel = f"{cur_vol / avg_vol:.2f}x" if avg_vol else "n/a"
    lines.append(f"VOLUME: current={_fmt(cur_vol, 2)}  avg(20)={_fmt(avg_vol, 2)}  relative={rel}")

    # Key levels
    r_str = " | ".join(_fmt(lv) for lv in sorted(levels["resistance"])) or "none"
    s_str = " | ".join(_fmt(lv) for lv in sorted(levels["support"])) or "none"
    lines.append(f"KEY LEVELS: R: {r_str}  S: {s_str}")

    return "\n".join(lines)
