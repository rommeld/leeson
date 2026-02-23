# Session Context

**Session ID:** fdcce120-ed9b-4411-b8ac-66c75563050e

**Commit Message:** Implement the following plan:

# Plan: Rewrite README.md Installation an

## Prompt

Implement the following plan:

# Plan: Rewrite README.md Installation and Usage Sections

## Context

The current README has a minimal "Usage" section (lines 63-90) that only lists `cargo` commands and a brief Python extension example. It lacks prerequisites, configuration, environment variables, risk config setup, agent setup, simulation mode, TUI keybindings, and platform support — all of which are important for someone setting up the project.

The architecture diagram on line 13 also says "PyO3" for the agent connection, but agents actually communicate via JSON-lines over stdin/stdout pipes. PyO3 is a separate feature for using Rust types from Python directly.

## Changes

**File:** `README.md`

Replace the "Usage" section (lines 63-90) and fix the architecture diagram. Keep the title, description, and project structure sections unchanged.

### New content outline

1. **Fix architecture diagram** (line 11): Change `PyO3` label to `JSON-lines` or `stdio` since agents communicate via pipes, not PyO3. Add a note that PyO3 is available as a separate feature.

2. **Prerequisites** section:
   - Rust (stable, edition 2024)
   - Python 3.12+ and `uv` (for agents)
   - macOS (ARM64), Linux (x86_64), or Windows (x86_64)

3. **Configuration** section:
   - Environment variables table: `KRAKEN_API_KEY`, `KRAKEN_API_SECRET`, `KRAKEN_WEBSOCKET_URL`, `LEESON_SIMULATION`
   - Note: credentials can also be entered at runtime via the TUI (`a` key) or stored in macOS Keychain
   - `risk.json` — must exist in working directory, explain its purpose and format

4. **Building and Running** section:
   - `cargo build` / `cargo run`
   - Simulation mode: `LEESON_SIMULATION=true cargo run`
   - Agent spawning happens automatically from the TUI

5. **TUI Key Bindings** section:
   - Table of key bindings for navigation, agents, overlays

6. **Python Extension** section (keep but improve):
   - `maturin develop --release` with feature explanation
   - Brief usage example

7. **Development** section:
   - `cargo test`, `cargo clippy`, `cargo fmt`

## Verification

- Read the final README.md to confirm formatting and accuracy
- Ensure no information contradicts the actual source code


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.REDACTED.jsonl

## Summary

The README looks good. All sections are in place, the architecture diagram is corrected, and the content is accurate. Here's a summary of the changes:

**Architecture diagram** — Changed `PyO3` label to `JSON-lines (stdin/stdout)` to reflect how agents actually communicate.

**Description** — Updated to mention JSON-lines over stdin/stdout instead of PyO3.

**New sections added:**
- **Prerequisites** — Rust 1.85+, Python 3.12+ with uv, supported platforms
- **Configuration** — Environment variables table (7 vars), `risk.json` format with example, `agent_risk.json` format with example
- **Building and Running** — Build commands including simulation mode, agent spawning explanation
- **TUI Key Bindings** — Organized by context (Global, Agent Tab Normal/Insert, Trading Pair Tab, Confirm Overlay)
- **Python Extension** — Improved with `--features` flags, `from decimal import Decimal`, and feature flag explanations
- **Development** — Test, lint, format, and check commands

## Key Actions

- - **Read**: /Users/denny/Documents/repos/leeson/README.md
- - **Task**: Gather README details from codebase
- - **Read**: /Users/denny/Documents/repos/leeson/risk.json
- - **Glob**: **/agent_risk.json
- - **Read**: /Users/denny/Documents/repos/leeson/agent_risk.json
- - **Edit**: /Users/denny/Documents/repos/leeson/README.md
- - **Edit**: /Users/denny/Documents/repos/leeson/README.md
- - **Edit**: /Users/denny/Documents/repos/leeson/README.md
- - **Read**: /Users/denny/Documents/repos/leeson/README.md
