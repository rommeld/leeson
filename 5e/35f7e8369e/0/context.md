# Session Context

**Session ID:** 2cacc8e7-e29a-40bb-897f-27dfb3d81913

**Commit Message:** Implement the following plan:

# API Keys Input Overlay for TUI

## Cont

## Prompt

Implement the following plan:

# API Keys Input Overlay for TUI

## Context

API keys (Fireworks AI, Kraken key, Kraken secret) are currently only loadable via environment variables at startup. The user wants a TUI overlay to manage these keys at runtime with persistent, secure storage (macOS Keychain via `keyring` crate). Three scenarios must be handled: adding new keys, overwriting existing keys, and discarding when keys already exist (with user notice).

## Implementation

### 1. Add `keyring` dependency

**File:** `Cargo.toml`

Add `keyring = "3"` to `[dependencies]`.

### 2. Create credentials module

**File:** `src/credentials.rs` (new)

- Enum `CredentialKey` with variants: `FireworksApiKey`, `KrakenApiKey`, `KrakenApiSecret`
  - Method `keyring_id() -> &'static str` for keychain entry names
  - Method `env_var() -> &'static str` returning `FIREWORKS_API_KEY`, `KRAKEN_API_KEY`, `KRAKEN_API_SECRET`
  - Method `label() -> &'static str` returning display names for the TUI
- `fn load(key: &CredentialKey) -> Option<Zeroizing<String>>` — reads from keychain, returns `None` on `NoEntry`
- `fn save(key: &CredentialKey, value: &str) -> crate::Result<()>` — saves to keychain
- `fn is_set(key: &CredentialKey) -> bool` — checks if a key exists in keychain
- `fn populate_env_from_keychain()` — for each key, if env var is not set but keychain has a value, set the env var. Called at startup before `fetch_config()`.

All credential values wrapped in `Zeroizing<String>`.

**File:** `src/lib.rs` — add `pub mod credentials;`

### 3. Load from keychain at startup

**File:** `src/main.rs`

Before the existing `fetch_config()` call, add:
```rust
credentials::populate_env_from_keychain();
```

This allows keys stored in the keychain to be used transparently by the existing env-var-based config flow.

### 4. Add TUI state

**File:** `src/tui/app.rs`

Add `Mode::ApiKeys` variant to the `Mode` enum.

Add new structs:
```rust
pub struct ApiKeysEditState {
    pub selected: usize,              // 0=Fireworks, 1=Kraken Key, 2=Kraken Secret
    pub editing: bool,                // Whether field is being edited
    pub input: String,                // Edit buffer
    pub cursor: usize,                // Cursor position
    pub fields: [ApiKeyField; 3],     // Per-field state
}

pub struct ApiKeyField {
    pub was_set: bool,                // Was this key already set when overlay opened?
    pub new_value: Option<String>,    // New value entered (None = not edited)
}
```

Constants:
- `FIELD_COUNT = 3`
- Methods: `field_label(index)`, `field_status(index)`, `new()`

Add to `App`:
```rust
pub api_keys_edit: Option<ApiKeysEditState>,
```

### 5. Add overlay rendering

**File:** `src/tui/ui.rs`

Add `render_api_keys_overlay(frame, state)` following the `render_risk_edit_overlay` pattern:
- Centered rect (60% width, 50% height)
- `Clear` widget to erase background
- Yellow border, title " API Keys "
- For each field (3 total):
  - Selection marker `▸` for active field
  - Label (e.g., "Fireworks API Key")
  - Status: `[not set]` (gray) or `[set]` (green) based on `was_set`
  - If field has `new_value`: show `[new: ********]`
  - If currently editing: show actual text with cursor (so user can verify paste)
- Help text at bottom: `[j/k] navigate  [Enter] edit  [s] save  [Esc] cancel`
- When editing: `[Enter] confirm  [Esc] cancel edit`

Hook into `render()`:
```rust
if app.mode == Mode::ApiKeys && let Some(ref state) = app.api_keys_edit {
    render_api_keys_overlay(frame, state);
}
```

### 6. Add event handling

**File:** `src/tui/event.rs`

Add `Action::SaveApiKeys` variant containing the 3 optional new values (wrapped in `Zeroizing<String>`).

Add `a` key binding in `handle_normal_mode()`:
```rust
KeyCode::Char('a') => {
    // Build ApiKeysEditState, checking which keys are already set
    app.api_keys_edit = Some(ApiKeysEditState::new());
    app.mode = Mode::ApiKeys;
    None
}
```

Add `Mode::ApiKeys` dispatch in `handle_key()` (before the global Esc handler, same pattern as `Mode::RiskEdit`):
```rust
if app.mode == Mode::ApiKeys {
    return handle_api_keys_mode(app, key);
}
```

Add `handle_api_keys_mode(app, key)`:
- **Not editing**: j/k navigate, Enter/i start editing, s save, Esc close
- **Editing**: character input (any printable char, not just digits), Backspace, Delete, Left, Right, Home, End, Enter to confirm field, Esc to cancel field edit
- On `s` (save):
  - Validate: if Kraken key is set but secret is not (or vice versa), show error and return
  - Build `Action::SaveApiKeys` with the new values
  - For fields where `was_set && new_value.is_none()`: show notice via `app.show_error("N keys unchanged (already set)")`
  - Close overlay, return to Normal mode

### 7. Add credential update command to ConnectionManager

**File:** `src/websocket/connection.rs`

Add to `ConnectionCommand`:
```rust
UpdateCredentials {
    api_key: Option<Zeroizing<String>>,
    api_secret: Option<Zeroizing<String>>,
},
```

Add to `DisconnectReason` (or equivalent):
```rust
CredentialsUpdated,
```

In `read_loop()`, handle the new command:
- Update `self.api_key` and `self.api_secret`
- Return `DisconnectReason::CredentialsUpdated`

In the outer `run()` loop, handle `CredentialsUpdated`:
- Skip backoff (like `TokenExpired`)
- Immediately reconnect with new credentials
- The existing `has_credentials()` check determines whether to connect to private endpoint

### 8. Handle SaveApiKeys action in main.rs

**File:** `src/main.rs`

In the action match block, handle `Action::SaveApiKeys`:
1. For each credential with a new value:
   - Save to keychain via `credentials::save()`
   - Set env var via `std::env::set_var()`
   - Track what changed (new vs overwritten)
2. If Kraken key or secret changed:
   - Load the full credential pair (new values + any existing unchanged values from keychain)
   - Send `ConnectionCommand::UpdateCredentials { api_key, api_secret }` via `cmd_tx`
3. Show appropriate notice via `app.show_error()`:
   - "API keys saved" (for new keys)
   - "API keys updated" (for overwrites)
   - Include count of unchanged keys if any were skipped

### 9. Update keybindings help

**File:** `src/tui/tabs/agent.rs` (keybindings section at bottom)

Add `[a] api keys` to the normal-mode keybindings display.

## Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` | Add `keyring` dependency |
| `src/lib.rs` | Add `pub mod credentials` |
| `src/credentials.rs` | **New** — keychain operations |
| `src/main.rs` | Load from keychain at startup, handle `SaveApiKeys` action |
| `src/tui/app.rs` | Add `Mode::ApiKeys`, `ApiKeysEditState`, `ApiKeyField`, field on `App` |
| `src/tui/ui.rs` | Add `render_api_keys_overlay()`, hook into `render()` |
| `src/tui/event.rs` | Add `Action::SaveApiKeys`, `a` binding, `handle_api_keys_mode()` |
| `src/websocket/connection.rs` | Add `UpdateCredentials` command, handle reconnection |
| `src/tui/tabs/agent.rs` | Add `[a]` to keybindings help |

## Verification

1. `cargo check` — ensure everything compiles
2. `cargo clippy` — no warnings
3. `cargo test` — existing tests pass
4. Manual testing:
   - Start app without env vars → press `a` → all fields show `[not set]` → enter all 3 keys → press `s` → keys saved to keychain, connection established
   - Restart app without env vars → keys loaded from keychain automatically
   - Press `a` again → fields show `[set]` → close with Esc → notice shown, nothing changed
   - Press `a` → edit one field → press `s` → overwritten key saved, unchanged keys noted
   - Enter Kraken key without secret → error shown on save


If you need specific details from before exiting plan mode (like exact code snippets, error messages, or content you generated), read the full transcript at: /Users/denny/.REDACTED.jsonl

## Summary

Implementation complete. Here's a summary of all changes:

**Files modified:**

| File | Change |
|------|--------|
| `Cargo.toml` | Added `keyring = "3"` dependency |
| `src/lib.rs` | Added `pub mod credentials;` |
| `src/credentials.rs` | **New** — `CredentialKey` enum, `load`/`save`/`is_set`/`populate_env_from_keychain` functions using macOS Keychain via the `keyring` crate |
| `src/main.rs` | Calls `populate_env_from_keychain()` at startup before `fetch_config()`, handles `Action::SaveApiKeys` to persist keys + trigger reconnection |
| `src/tui/app.rs` | Added `Mode::ApiKeys`, `ApiKeysEditState`, `ApiKeyField`, `FieldStatus`, and `api_keys_edit` field on `App` |
| `src/tui/ui.rs` | Added `render_api_keys_overlay()` with centered dialog, field status indicators, and help text |
| `src/tui/event.rs` | Added `Action::SaveApiKeys`, `a` keybinding, `handle_api_keys_mode()`, `handle_api_key_field_edit()` |
| `src/websocket/connection.rs` | Added `UpdateCredentials` command variant and `CredentialsUpdated` disconnect reason with no-backoff reconnection |
| `src/tui/tabs/agent.rs` | Added `[a]api keys` to normal-mode keybindings and `Mode::ApiKeys` help text |

**Verification:** `cargo check`, `cargo clippy` (0 warnings), `cargo test` (138 passed) all clean.

## Key Actions

- - **Read**: /Users/denny/Documents/repos/leeson/Cargo.toml
- - **Read**: /Users/denny/Documents/repos/leeson/src/lib.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/main.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/app.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/event.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/ui.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/websocket/connection.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/tui/tabs/agent.rs
- - **TaskCreate**: Add `keyring = "3"` to [dependencies] in Cargo.toml
- - **TaskCreate**: Create src/credentials.rs with CredentialKey enum, load/save/is_set/populate_env_from_keychain functions
