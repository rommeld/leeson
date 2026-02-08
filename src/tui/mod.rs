//! Terminal User Interface for the Leeson trading platform.
//!
//! Provides a Ratatui-based TUI for real-time market data display,
//! order management, and agent interaction.

pub mod app;
pub mod components;
pub mod event;
pub mod input;
pub mod tabs;
pub mod terminal;
pub mod ui;

pub use app::App;
pub use event::{Event, Message};
pub use terminal::{Tui, restore_terminal, setup_terminal};
pub use ui::render;
