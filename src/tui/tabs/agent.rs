//! Agent tab layout and rendering.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::tui::app::{App, Focus, Mode};
use crate::tui::components::{status_bar, tab_bar};

/// Renders the Agent tab.
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Main vertical layout
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab bar
            Constraint::Length(1), // Status bar
            Constraint::Min(6),    // Agent outputs
            Constraint::Length(7), // Account overview
            Constraint::Length(7), // Open orders
            Constraint::Length(7), // Executed trades
            Constraint::Length(5), // Pair selector
            Constraint::Length(3), // Agent input
            Constraint::Length(1), // Keybindings help
        ])
        .split(area);

    // Tab bar
    tab_bar::render(frame, main_layout[0], app);

    // Status bar
    status_bar::render(frame, main_layout[1], app);

    // Agent output panels (3 columns)
    render_agent_outputs(frame, main_layout[2], app);

    // Account overview
    render_account_overview(frame, main_layout[3], app);

    // Open orders
    render_open_orders(frame, main_layout[4], app);

    // Executed trades
    render_executed_trades(frame, main_layout[5], app);

    // Pair selector
    render_pair_selector(frame, main_layout[6], app);

    // Agent input
    render_agent_input(frame, main_layout[7], app);

    // Keybindings help
    render_keybindings(frame, main_layout[8], app);
}

/// Renders the three agent output panels.
fn render_agent_outputs(frame: &mut Frame, area: Rect, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    let titles = [" User Agent ", " Market Agent ", " Risk & Execution "];

    for (i, col) in columns.iter().enumerate() {
        let focus = match i {
            0 => Focus::AgentOutput1,
            1 => Focus::AgentOutput2,
            _ => Focus::AgentOutput3,
        };
        let is_focused = app.focus == focus;

        // Agent 1 gets a special highlight since it's the interactive one
        let border_style = if i == 0 && app.focus == Focus::AgentInput {
            Style::default().fg(Color::Yellow)
        } else if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .title(titles[i])
            .borders(Borders::ALL)
            .border_style(border_style);

        let items: Vec<ListItem> = app.agent_outputs[i]
            .iter()
            .map(|line| ListItem::new(line.as_str()))
            .collect();

        let list = List::new(items).block(block);

        frame.render_widget(list, *col);
    }
}

/// Renders the account overview panel with asset balances.
fn render_account_overview(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Account Overview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    // Left column: Asset balances breakdown
    let mut left_lines: Vec<Line> = Vec::new();

    // Header
    left_lines.push(Line::from(Span::styled(
        format!(
            "{:<8} {:>12} {:>10} {:>10}",
            "Asset", "Total", "Spot", "Earn"
        ),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )));

    // Sort assets: USD first, then alphabetically
    let mut assets: Vec<_> = app.asset_balances.values().collect();
    assets.sort_by(|a, b| {
        if a.asset == "USD" {
            std::cmp::Ordering::Less
        } else if b.asset == "USD" {
            std::cmp::Ordering::Greater
        } else {
            a.asset.cmp(&b.asset)
        }
    });

    // Show up to 4 assets (limited by panel height)
    for balance in assets.iter().take(4) {
        let color = if balance.asset == "USD" {
            Color::Cyan
        } else {
            Color::White
        };

        left_lines.push(Line::from(vec![
            Span::styled(format!("{:<8} ", balance.asset), Style::default().fg(color)),
            Span::styled(
                format!("{:>12.4} ", balance.total),
                Style::default().fg(color),
            ),
            Span::raw(format!("{:>10.4} ", balance.spot)),
            Span::raw(format!("{:>10.4}", balance.earn)),
        ]));
    }

    if app.asset_balances.is_empty() {
        left_lines.push(Line::from(Span::styled(
            "No balance data",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let left_para = Paragraph::new(left_lines);
    frame.render_widget(left_para, layout[0]);

    // Right column: P&L and positions
    let pnl_today_color = if app.pnl_today >= rust_decimal::Decimal::ZERO {
        Color::Green
    } else {
        Color::Red
    };
    let pnl_total_color = if app.pnl_total >= rust_decimal::Decimal::ZERO {
        Color::Green
    } else {
        Color::Red
    };

    let open_positions: usize = app.open_orders.values().map(|v| v.len()).sum();
    let total_assets = app.asset_balances.len();

    let right_text = vec![
        Line::from(vec![
            Span::raw("P&L Today: "),
            Span::styled(
                format!("${:.2}", app.pnl_today),
                Style::default().fg(pnl_today_color),
            ),
        ]),
        Line::from(vec![
            Span::raw("P&L Total: "),
            Span::styled(
                format!("${:.2}", app.pnl_total),
                Style::default().fg(pnl_total_color),
            ),
        ]),
        Line::from(vec![
            Span::raw("Open Positions: "),
            Span::styled(
                format!("{}", open_positions),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::raw("Total Assets: "),
            Span::styled(
                format!("{}", total_assets),
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    let right_para = Paragraph::new(right_text);
    frame.render_widget(right_para, layout[1]);
}

/// Renders the open orders table (all pairs).
fn render_open_orders(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::OpenOrdersAll;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let total_orders: usize = app.open_orders.values().map(|v| v.len()).sum();
    let title = format!(" Open Orders ({}) ", total_orders);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Header
    let header = Line::from(vec![Span::styled(
        format!(
            "{:<10} {:<6} {:>10} {:>12}",
            "Symbol", "Side", "Qty", "Price"
        ),
        Style::default().add_modifier(Modifier::BOLD),
    )]);

    let mut lines = vec![header];

    // Collect all open orders across all symbols
    let mut all_orders: Vec<_> = app
        .open_orders
        .iter()
        .flat_map(|(symbol, orders)| orders.iter().map(move |o| (symbol, o)))
        .collect();

    // Sort by symbol
    all_orders.sort_by(|a, b| a.0.cmp(b.0));

    // Show orders (limited by panel height)
    let max_rows = inner.height.saturating_sub(1) as usize;
    for (symbol, order) in all_orders.iter().take(max_rows) {
        let side_color = if order.side.to_uppercase() == "BUY" {
            Color::Green
        } else {
            Color::Red
        };

        let price = order.limit_price.unwrap_or(rust_decimal::Decimal::ZERO);

        lines.push(Line::from(vec![
            Span::raw(format!("{:<10} ", symbol)),
            Span::styled(
                format!("{:<6} ", order.side.to_uppercase()),
                Style::default().fg(side_color),
            ),
            Span::raw(format!("{:>10.4} ", order.order_qty)),
            Span::raw(format!("{:>12.2}", price)),
        ]));
    }

    if all_orders.is_empty() {
        lines.push(Line::from(Span::styled(
            "No open orders",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

/// Renders the executed trades table.
fn render_executed_trades(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::ExecutedTradesAll;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Executed Trades (All Pairs) ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Header
    let header = Line::from(vec![Span::styled(
        format!(
            "{:<10} {:<10} {:<6} {:<10} {:<12} {:<10}",
            "Time", "Symbol", "Side", "Qty", "Price", "P&L"
        ),
        Style::default().add_modifier(Modifier::BOLD),
    )]);

    let mut lines = vec![header];

    // Trade rows
    for trade in app.executed_trades_all.iter().rev().take(5) {
        let side_color = if trade.side.to_uppercase() == "BUY" {
            Color::Green
        } else {
            Color::Red
        };

        let pnl_str = trade
            .pnl
            .map(|p| format!("{:+.2}", p))
            .unwrap_or_else(|| "-".to_string());

        lines.push(Line::from(vec![
            Span::raw(format!(
                "{:<10} ",
                &trade.timestamp[..10.min(trade.timestamp.len())]
            )),
            Span::raw(format!("{:<10} ", trade.symbol)),
            Span::styled(
                format!("{:<6} ", trade.side),
                Style::default().fg(side_color),
            ),
            Span::raw(format!("{:<10} ", trade.qty)),
            Span::raw(format!("{:<12} ", trade.price)),
            Span::raw(pnl_str),
        ]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

/// Renders the pair selector.
fn render_pair_selector(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::PairSelector;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Pair Selector ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Create a grid of pairs (4 per row)
    let pairs_per_row = 4;
    let mut lines: Vec<Line> = Vec::new();

    for (i, chunk) in app.available_pairs.chunks(pairs_per_row).enumerate() {
        let spans: Vec<Span> = chunk
            .iter()
            .enumerate()
            .map(|(j, pair)| {
                let idx = i * pairs_per_row + j;
                let is_selected = app.is_pair_selected(pair);
                let is_cursor = idx == app.pair_selector_index && is_focused;

                let checkbox = if is_selected { "[x]" } else { "[ ]" };

                let style = if is_cursor {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else if is_selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                Span::styled(format!("{} {:<12}", checkbox, pair), style)
            })
            .collect();

        lines.push(Line::from(spans));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

/// Renders the agent input field for Agent 1.
fn render_agent_input(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::AgentInput;
    let is_insert = app.mode == Mode::Insert && is_focused;

    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if is_insert {
        " Agent 1 Input (INSERT) "
    } else {
        " Agent 1 Input "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let prompt = "> ";
    let text = format!("{}{}", prompt, app.agent_input);

    let para = Paragraph::new(text).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);

    // Show cursor in insert mode
    if is_insert {
        let cursor_x = inner.x + prompt.len() as u16 + app.agent_input_cursor as u16;
        let cursor_y = inner.y;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

/// Renders the keybindings help line.
fn render_keybindings(frame: &mut Frame, area: Rect, app: &App) {
    let help = match app.mode {
        Mode::Insert => "[Esc]normal [Enter]send to Agent 1",
        Mode::Normal => {
            "[Tab]switch tab [Space]toggle pair [i]Agent 1 input [1-3]focus agent [r]risk [?]help [q]quit"
        }
        Mode::Confirm => "[y]yes [n]no",
        Mode::RiskEdit => "[j/k]navigate [Space]toggle [Enter]edit [s]save [Esc]cancel",
    };

    let para = Paragraph::new(help).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(para, area);
}
