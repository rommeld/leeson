//! Kraken WebSocket V2 client library.
//!
//! Provides typed models and async WebSocket functions for subscribing to
//! Kraken's public market data channels (ticker, book, trades, candles,
//! instruments, and level-3 orders).

pub mod agent;
pub mod auth;
pub mod config;
pub mod error;
pub mod models;
#[cfg(feature = "python")]
mod python;
pub mod tls;
pub mod tui;
pub mod websocket;

pub use error::{LeesonError, Result};
