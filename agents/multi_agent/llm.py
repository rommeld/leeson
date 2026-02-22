"""Fireworks AI model factory for pydantic-ai agents.

Uses FIREWORKS_API_KEY env var (set externally).
MiniMax M2.5 is the default model — SOTA for agentic tool use,
with automatic prompt caching on Fireworks ($0.03/M cached input).
"""

from __future__ import annotations

from pydantic_ai.models.openai import OpenAIChatModel
from pydantic_ai.providers.fireworks import FireworksProvider

DEFAULT_MODEL = "accounts/fireworks/models/minimax-m2p5"


def create_model(model_name: str = DEFAULT_MODEL) -> OpenAIChatModel:
    """Create a Fireworks-backed chat model for pydantic-ai."""
    return OpenAIChatModel(model_name, provider=FireworksProvider())
