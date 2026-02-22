//! Status bar component.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::app::{App, ConnectionStatus};

/// Renders the status bar.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let status_color = match app.connection_status {
        ConnectionStatus::Connected => Color::Green,
        ConnectionStatus::Connecting | ConnectionStatus::Reconnecting => Color::Yellow,
        ConnectionStatus::Disconnected => Color::Red,
    };

    let auth_label = if !app.authenticated {
        Span::styled(" No Auth ", Style::default().fg(Color::DarkGray))
    } else if app.private_connected {
        Span::styled(" Auth ", Style::default().fg(Color::Green))
    } else {
        Span::styled(" Auth Down ", Style::default().fg(Color::Yellow))
    };

    // Show USD balance if available
    let balance_span = if let Some(usd) = app.asset_balances.get("USD") {
        Span::styled(
            format!(" ${:.2} ", usd.total),
            Style::default().fg(Color::Cyan),
        )
    } else {
        Span::raw("")
    };

    let tab_info = format!(" {}/{} ", app.active_tab + 1, app.tabs.len());

    let error_span = if let Some(ref error) = app.error_message {
        Span::styled(
            format!(" {} ", error.message),
            Style::default().fg(Color::Red),
        )
    } else {
        Span::raw("")
    };

    // Simulation badge: yellow background with P&L and trade count
    let sim_spans: Vec<Span> = if app.simulation {
        let total_pnl = app.sim_stats.realized_pnl + app.sim_stats.unrealized_pnl;
        let pnl_color = if total_pnl >= rust_decimal::Decimal::ZERO {
            Color::Green
        } else {
            Color::Red
        };
        vec![
            Span::styled(" SIM ", Style::default().fg(Color::Black).bg(Color::Yellow)),
            Span::styled(
                format!(" P&L:{:+.2} ", total_pnl),
                Style::default().fg(pnl_color),
            ),
            Span::styled(
                format!("#{} ", app.sim_stats.trade_count),
                Style::default().fg(Color::White),
            ),
            Span::raw("│"),
        ]
    } else {
        vec![]
    };

    let mut spans = sim_spans;
    spans.extend(vec![
        Span::styled(
            format!(" {} ", app.connection_status.label()),
            Style::default().fg(status_color),
        ),
        Span::raw("│"),
        auth_label,
        Span::raw("│"),
        balance_span,
        Span::raw("│"),
        error_span,
        Span::raw(format!(
            "{:>width$}",
            tab_info,
            width = area.width.saturating_sub(45) as usize
        )),
    ]);

    let line = Line::from(spans);

    let para = Paragraph::new(line).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(para, area);
}
