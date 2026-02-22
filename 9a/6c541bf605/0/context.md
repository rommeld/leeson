# Session Context

**Session ID:** 6c162f21-faa6-49e5-8a5f-64cc40a0547c

**Commit Message:** What is the env variable for the fireworks AI key

## Prompt

what is the env variable for the fireworks AI key?

## Summary

The environment variable is **`FIREWORKS_API_KEY`**.

It's used in `agents/multi_agent/llm.py` via the `FireworksProvider()` from pydantic-ai, which reads it automatically.

## Key Actions

- - **Grep**: (?i)fireworks|FIREWORKS
