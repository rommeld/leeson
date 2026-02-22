//! Main UI rendering coordinator.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use super::app::{ApiKeysEditState, App, FieldStatus, Mode, RiskEditState, Tab};
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

    // Render risk parameters edit overlay
    if app.mode == Mode::RiskEdit
        && let Some(ref state) = app.risk_edit
    {
        render_risk_edit_overlay(frame, state);
    }

    // Render API keys edit overlay
    if app.mode == Mode::ApiKeys
        && let Some(ref state) = app.api_keys_edit
    {
        render_api_keys_overlay(frame, state);
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

/// Renders the risk parameters edit overlay.
fn render_risk_edit_overlay(frame: &mut Frame, state: &RiskEditState) {
    let area = frame.area();
    let dialog = centered_rect(50, 40, area);

    frame.render_widget(Clear, dialog);

    let mut lines = vec![
        Line::from(Span::styled(
            "Agent Risk Parameters",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for i in 0..RiskEditState::FIELD_COUNT {
        let is_selected = i == state.selected;
        let label = RiskEditState::field_label(i);

        let value_str = if state.editing && is_selected {
            format!("{}▏", state.input)
        } else {
            state.field_value(i)
        };

        let label_style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let value_style = if state.editing && is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::UNDERLINED)
        } else if is_selected {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        let marker = if is_selected { "▸ " } else { "  " };

        lines.push(Line::from(vec![
            Span::styled(marker, label_style),
            Span::styled(format!("{label}: "), label_style),
            Span::styled(value_str, value_style),
        ]));
    }

    lines.push(Line::from(""));

    let help = if state.editing {
        vec![
            Span::styled(
                "[Enter] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("confirm  "),
            Span::styled(
                "[Esc] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("cancel edit"),
        ]
    } else {
        vec![
            Span::styled(
                "[j/k] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("navigate  "),
            Span::styled(
                "[Space] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("toggle  "),
            Span::styled(
                "[Enter] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("edit  "),
            Span::styled(
                "[s] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("save  "),
            Span::styled(
                "[Esc] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("cancel"),
        ]
    };
    lines.push(Line::from(help));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Risk Parameters ");

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, dialog);
}

/// Renders the API keys edit overlay.
fn render_api_keys_overlay(frame: &mut Frame, state: &ApiKeysEditState) {
    let area = frame.area();
    let dialog = centered_rect(60, 50, area);

    frame.render_widget(Clear, dialog);

    let mut lines = vec![
        Line::from(Span::styled(
            "API Keys",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for i in 0..ApiKeysEditState::FIELD_COUNT {
        let is_selected = i == state.selected;
        let label = ApiKeysEditState::field_label(i);
        let status = state.field_status(i);

        let marker = if is_selected { "▸ " } else { "  " };

        let label_style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let status_span = match status {
            FieldStatus::NotSet => Span::styled(" [not set]", Style::default().fg(Color::DarkGray)),
            FieldStatus::Set => Span::styled(" [set]", Style::default().fg(Color::Green)),
            FieldStatus::NewValue => Span::styled(" [new: ********]", Style::default().fg(Color::Yellow)),
        };

        if state.editing && is_selected {
            // Show actual input text with cursor so user can verify paste
            let value_style = Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::UNDERLINED);

            lines.push(Line::from(vec![
                Span::styled(marker, label_style),
                Span::styled(format!("{label}: "), label_style),
                Span::styled(format!("{}▏", state.input), value_style),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(marker, label_style),
                Span::styled(format!("{label}:"), label_style),
                status_span,
            ]));
        }
    }

    lines.push(Line::from(""));

    let help = if state.editing {
        vec![
            Span::styled(
                "[Enter] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("confirm  "),
            Span::styled(
                "[Esc] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("cancel edit"),
        ]
    } else {
        vec![
            Span::styled(
                "[j/k] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("navigate  "),
            Span::styled(
                "[Enter] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("edit  "),
            Span::styled(
                "[s] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("save  "),
            Span::styled(
                "[Esc] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("cancel"),
        ]
    };
    lines.push(Line::from(help));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" API Keys ");

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
