# Session Context

**Session ID:** d3716988-e97a-4b46-85a9-82f65b9a1fc3

**Commit Message:** Enforce per-collection capacity limits. Use binary search insertion inst

## Prompt

Enforce per-collection capacity limits. Use binary search insertion instead of sort-after-push for book levels. Check @src/tui/app.rs and @src/tui/event.rs

## Summary

Everything compiles cleanly, tests pass, and clippy is happy. Here's a summary of the changes:

**`app.rs`** — Added `MAX_BOOK_DEPTH` constant (1000) as a safety cap for order book levels per side.

**`event.rs`** — Three changes:

1. **Binary search insertion for book levels** — Replaced `push` + `sort_by` with `binary_search_by` + `insert` for both bids (descending) and asks (ascending). This is O(n) per insertion instead of O(n log n) for the full sort.

2. **Book depth cap** — After both snapshot and incremental update paths, `bids` and `asks` are truncated to `MAX_BOOK_DEPTH` (1000). Placed after checksum validation so checksums remain accurate under normal operation; the cap is a defensive bound against unbounded growth.

3. **Open orders cap** — Added `MAX_OPEN_ORDERS_PER_SYMBOL` (200). When a new order would exceed the limit, the oldest entry is dropped before pushing.

## Key Actions

- - **Edit**: /Users/denny/Documents/repos/leeson/src/tui/app.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Edit**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Bash**: Check compilation
- - **Bash**: Run event module tests
- - **Bash**: Run clippy linter
