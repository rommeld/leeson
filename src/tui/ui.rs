//! Main UI rendering coordinator.

use ratatui::Frame;

use super::app::{App, Tab};
use super::tabs::{agent, trading_pair};

/// Renders the entire application UI.
pub fn render(frame: &mut Frame, app: &App) {
    match app.current_tab() {
        Tab::Agent => agent::render(frame, app),
        Tab::TradingPair(symbol) => trading_pair::render(frame, app, symbol),
    }
}
