# Project

This project is a crypto currency trading platform built as a hybrid Rust/Python system. The Rust layer provides two core capabilities: a terminal user interface (TUI) for real-time operator interaction and monitoring, and a WebSocket client that maintains a persistent connection to the Kraken exchange for market data and order execution. The Python layer implements autonomous trading agents that leverage Anthropic's Claude API for decision-making and strategy execution. The architecture bridges Rust and Python across a process boundary, with Rust handling the performance-critical, low-latency exchange connectivity and rendering, while Python provides the flexible, AI-driven agent logic. Operators interact with and oversee the agents through the TUI, which serves as the primary control surface for the system.

# Commands

| Command                 | Purpose                                                                 |
| ----------------------- | ----------------------------------------------------------------------- |
| `cargo build`           | Compile the project in debug mode                                       |
| `cargo build --release` | Compile with optimizations for production                               |
| `cargo run`             | Build and execute the binary                                            |
| `cargo test`            | Run all unit tests, integration tests, and doc-tests                    |
| `cargo test <name>`     | Run tests matching a specific name or pattern                           |
| `cargo clippy`          | Run the linter to catch common mistakes and suggest improvements        |
| `cargo fmt`             | Format code according to Rust style guidelines                          |
| `cargo fmt --check`     | Verify formatting without modifying files (useful in CI)                |
| `cargo doc --open`      | Generate and open documentation for the project and dependencies        |
| `cargo update`          | Update dependencies to newest compatible versions per Cargo.toml        |
| `cargo check`           | Fast compile check without producing binaries—useful during development |

# Code Style

- **Leverage the type system for correctness**: Use enums for state machines where variants are mutually exclusive. Use the newtype pattern (struct Miles(f64)) to enforce semantic distinctions at compile time. Prefer typestate patterns to make invalid states unrepresentable—methods only exist on valid state types.
- **Design traits intentionally**: Use associated types when there's one natural implementation per type; use generics when multiple implementations make sense. Keep traits object-safe when `dyn Trait` flexibility is needed (no -> Self returns, no generic methods).
- **Avoid bare** `unwrap()`: Prefer `expect("reason") to document assumptions if behavior is predictable. Use combinators like `unwrap_or_default()`, `unwrap_or_else(|| ...)`, `or ok_or_else(|| ...)` for recoverable cases. Reserve unwrap() for tests.
- **Prefer zero-cost abstractions**: Use iterator chains over manual loops—they compile to equivalent code with better optimization opportunities. Newtypes have no runtime overhead. Generics with trait bounds use static dispatch; reach for `dyn Trait` only when dynamic dispatch is genuinely needed.
- **Treat tests and docs as first-class**: Place unit tests in #[cfg(test)] modules alongside code. Write doc-tests in /// comments to keep examples synchronized. Document # Panics, # Errors, and # Safety sections where applicable.
- **Design errors intentionally**: Categorize errors by what callers can do (retry, skip, fail) rather than which component failed. Error messages should be lowercase, omit trailing punctuation, and describe only the immediate problem—let the source chain convey causality. Add meaningful context at module boundaries instead of blindly forwarding; ask "if this fails in production, what would I wish the log said?" Propagate with `?` and add context via `.context()` or `.with_context()`. Prefer preserving error chains over formatting errors inline. Library choice (_thiserror_, _anyhow_, or custom types) depends on whether callers need to match variants or just report—there's no one-size-fits-all rule.
- **Prefer** `crate::` **over** `super::`: Use absolute paths from the crate root for clarity and easier refactoring.
- **Use** `pub use` **sparingly**: Reserve re-exports for exposing dependencies so downstream consumers don't need direct dependencies—avoid it for internal module organization.
- **Avoid global state**: Skip `lazy_static!`, `OnceCell`, or similar patterns; prefer passing explicit context for shared state to keep dependencies visible and testing straightforward.
- **Keep dependencies current**: Regularly update to the newest crate versions to benefit from bug fixes, performance improvements, and security patches.
