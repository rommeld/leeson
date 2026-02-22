# Session Context

**Session ID:** 4aedcd96-8ff7-45bb-8115-a803591f5225

**Commit Message:** Where do I set the Kraken API to connect to my account

## Prompt

Where do I set the Kraken API to connect to my account?

## Summary

Set two environment variables before running leeson:

```sh
export KRAKEN_API_KEY="your-api-key"
export KRAKEN_API_SECRET="your-api-secret"
```

The config is loaded in `src/config.rs:55-83`. Both variables must be set together — setting only one is an error. If neither is set, the app runs in unauthenticated mode (public market data only, no trading).

You can optionally override the WebSocket endpoint:

```sh
export KRAKEN_WEBSOCKET_URL="wss://ws.kraken.com/v2"  # this is the default
```

The credentials are used in `src/auth.rs` to fetch a short-lived WebSocket token from Kraken's REST API (`GetWebSocketsToken`), which is then used for authenticated WebSocket channels (orders, executions, balances). The credentials are wrapped in `Zeroizing<String>` so they're scrubbed from memory on drop.

## Key Actions

- - **Grep**: api_key|api_secret|credentials|kraken.*auth|token.*kraken
- - **Read**: /Users/denny/Documents/repos/leeson/src/config.rs
- - **Read**: /Users/denny/Documents/repos/leeson/src/auth.rs
