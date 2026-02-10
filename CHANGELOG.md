# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Implement Kraken Websocket V2 client with all public channels (c439b5b)
- Add error handling to modulerized project (1f678aa)
- Add authenticated Level 3 orders channel support (4f31395)
- Add executions and balances channels from user data section (17bb9c2)
- Add orders channel to models and websocket.rs (e458cd5)
- Add the channel cancel_order (4a1032f)
- Add amend_orders channel to modify existing orders (3fd169f)
- Add channel cancel_all to be able to cancel all open orders (e247dda)
- Add Dead Man's Switch to intercept potential network malfunction (4788b81)
- Add the possibility to add a collection of trades by defining the minimum and maximum size of a batch (62a1177)
- Batch order can be cancelled (44dddf9)
- Add edition of an existing order which will be cancelled and replaced by modified order (8f7ff7d)
- Create base layout for TUI (9d14d82)
- Improve orderbook rendering with dynamic depth and spread percentage (4d5532b)
- Add ConnectionManager with reconnection and token refresh (2bdb6fc)
- Track order book snapshot history for best bid/ask over time (37736fb)
- Add order book spread history panel to trading pair view (3247908)
- Add balances channel model and subscription support (7862a2a)
- Split into public and private WebSocket connections (733dff2)
- Handle incremental order book updates and balance events (fb07c7d)
- Enhance TUI with asset balances, open orders panel, and layout improvements (22181f7)
- Support KRAKEN_API_KEY and KRAKEN_API_SECRET env vars (db044ca)

### Fixed

- Validate HTTP response status before parsing token body (7ecbd4b)
- Return proper error instead of panicking on log file creation failure (8ecf3a2)

### Changed

- Improve type safety and API ergonomics (1d152f4)
- Adjust project structure for easier readability (fded012)
- Change native TLS for a TLS certificate and reduce token log (7068c06)

### Security

- Zeroize API credentials in memory on drop (139c70d)
