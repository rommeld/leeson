# Session Context

**Session ID:** a1ee82b9-96fa-4694-8b47-77b958ac9fe3

**Commit Message:** Implement the following plan:

# Plan: Prompt for API Keys Before Connec

## Prompt

Implement the following plan:

# Plan: Prompt for API Keys Before Connecting

## Context

Currently the app loads credentials from the macOS Keychain/env vars at startup and immediately spawns the Kraken WebSocket connection and Python multi-agent system — even if credentials are missing. The user wants to be prompted for API keys **before** any connections are established.

The existing API keys overlay (`Mode::ApiKeys`) already provides a full credential editing UI. We just need to auto-open it on startup when credentials are missing, and defer service startup until it's dismissed.

## Approach

All changes are in **`src/main.rs`** only. The overlay, credential storage, and event handling code remain unchanged.

### 1. Detect missing credentials (after keychain/env loading)

After `populate_env_from_keychain()` and `fetch_config()`, check if any of the 3 credentials are absent:

```rust
let any_credentials_missing = CredentialKey::ALL
    .iter()
    .any(|key| std::env::var(key.env_var()).unwrap_or_default().is_empty());
```

### 2. Make early validation conditional

Only validate Kraken credentials when all creds are present (`!any_credentials_missing`). When missing, skip validation — it will happen after the overlay is dismissed.

### 3. Auto-open the API keys overlay

After creating `App` state, if credentials are missing:

```rust
if any_credentials_missing {
    app.api_keys_edit = Some(ApiKeysEditState::new());
    app.mode = Mode::ApiKeys;
}
```

### 4. Defer ConnectionManager and agent spawning

Introduce a `setup_complete` flag and hold `cmd_rx` in an `Option`:

- **If all creds present**: spawn ConnectionManager and agents immediately (current behavior)
- **If creds missing**: store `cmd_rx` in `deferred_cmd_rx: Option<Receiver>`, don't spawn anything yet
- Event reader and tick timer always start immediately (needed for TUI interaction)

### 5. Complete setup when overlay is dismissed

After the action-processing block in the event loop, check:

```rust
if !setup_complete && app.mode != Mode::ApiKeys {
    // Overlay was dismissed (Save or Esc)
    setup_complete = true;
    // Re-read credentials from env vars (SaveApiKeys updated them)
    // Validate Kraken credentials if present
    // Spawn ConnectionManager with deferred_cmd_rx.take()
    // Spawn multi-agent system
}
```

This handles both Save (credentials saved → connect with auth) and Esc (no credentials → public-only mode, agents may fail LLM calls).

### 6. Guard redundant UpdateCredentials

In the existing `SaveApiKeys` handler, wrap the `UpdateCredentials` send with `if setup_complete` to avoid sending credentials to a manager that hasn't been spawned yet (it will receive them at construction).

## Files to modify

- `src/main.rs` — restructure startup flow, add deferred spawn logic

## Verification

1. **No stored credentials**: `cargo run` → API keys overlay auto-opens → enter keys → Save → connection established, agents start
2. **Esc without saving**: overlay dismissed → public-only mode, agents fail gracefully
3. **All credentials in keychain**: `cargo run` → no overlay, immediate startup (existing behavior)
4. **Partial credentials**: overlay opens, user fills in missing ones
5. `cargo clippy` and `cargo test` pass


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.claude/projects/-Users-denny-Documents-repos-leeson/782e02a9-2619-434a-9c2c-013934343ca0.jsonl

## Summary

All done. Here's a summary of the changes made to `src/main.rs`:

1. **Detect missing credentials** (line 42-44): After `populate_env_from_keychain()` and `fetch_config()`, checks if any of the 3 credential env vars are empty.

2. **Conditional validation** (line 53): Skips Kraken credential validation when any credentials are missing — validation will happen after the overlay is dismissed.

3. **Auto-open API keys overlay** (lines 94-98): When credentials are missing, initializes the `ApiKeysEditState` and sets `Mode::ApiKeys` so the overlay appears immediately on first render.

4. **Deferred spawning** (lines 112-147): Introduces `setup_complete` and `deferred_cmd_rx`. When credentials are missing, the `ConnectionManager` and agents are **not** spawned — `cmd_rx` is held in an `Option` for later use.

5. **Deferred setup completion** (lines 496-547): After each event loop iteration, checks if the overlay was dismissed (`!setup_complete && app.mode != Mode::ApiKeys`). When true, re-reads credentials from keychain, validates if present, spawns the `ConnectionManager` and agents.

6. **Guarded `UpdateCredentials`** (line 479): The `SaveApiKeys` handler only sends `UpdateCredentials` to the connection manager when `setup_complete` is true, avoiding sends to a manager that hasn't been spawned yet.

## Key Actions

- - **Read**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Grep**: CredentialKey
- - **Grep**: ApiKeysEditState
- - **Grep**: CredentialKey
- - **Grep**: ApiKeysEditState
- - **Read**: /Users/denny/Documents/repos/leeson/src/credentials.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/app.rs
- - **TaskCreate**: Restructure src/main.rs to detect missing credentials, auto-open API keys overlay, and defer ConnectionManager + agent spawning until credentials are provided.
- - **TaskUpdate**: 
- - **Edit**: /Users/denny/Documents/repos/leeson/src/main.rs
