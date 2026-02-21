//! Main UI rendering coordinator.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use super::app::{App, Mode, Tab};
use super::tabs::{agent, trading_pair};

/// Renders the entire application UI.
pub fn render(frame: &mut Frame, app: &App) {
    match app.current_tab() {
        Tab::Agent => agent::render(frame, app),
        Tab::TradingPair(symbol) => trading_pair::render(frame, app, symbol),
    }

    // Render confirmation overlay on top of the current tab
    if app.mode == Mode::Confirm
        && let Some(ref pending) = app.pending_order
    {
        render_confirm_overlay(frame, app, pending);
    }
}

/// Renders a centered confirmation dialog overlay.
fn render_confirm_overlay(frame: &mut Frame, _app: &App, pending: &super::app::PendingOrder) {
    let area = frame.area();
    let dialog = centered_rect(60, 40, area);

    // Clear the area behind the dialog
    frame.render_widget(Clear, dialog);

    let params = &pending.params;
    let price_str = params
        .limit_price
        .map_or("market".to_string(), |p| p.to_string());

    let lines = vec![
        Line::from(Span::styled(
            "Order Confirmation Required",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Reason: ", Style::default().fg(Color::Yellow)),
            Span::raw(&pending.reason),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Symbol: ", Style::default().fg(Color::Cyan)),
            Span::raw(&params.symbol),
        ]),
        Line::from(vec![
            Span::styled("Side:   ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:?}", params.side)),
        ]),
        Line::from(vec![
            Span::styled("Type:   ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:?}", params.order_type)),
        ]),
        Line::from(vec![
            Span::styled("Qty:    ", Style::default().fg(Color::Cyan)),
            Span::raw(params.order_qty.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Price:  ", Style::default().fg(Color::Cyan)),
            Span::raw(price_str),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[Y] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Confirm   "),
            Span::styled(
                "[N] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Cancel"),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Confirm Order ");

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, dialog);
}

/// Returns a centered rectangle of the given percentage of the parent area.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
