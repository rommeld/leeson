//! Trading pair tab layout and rendering.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use rust_decimal::Decimal;

use crate::tui::app::{App, ChartType, Focus, OrdersView};
use crate::tui::components::{status_bar, tab_bar};

/// Renders a trading pair tab.
pub fn render(frame: &mut Frame, app: &App, symbol: &str) {
    let area = frame.area();

    // Main vertical layout
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab bar
            Constraint::Length(1), // Status bar
            Constraint::Length(1), // Ticker header
            Constraint::Min(10),   // Main content (order book + chart)
            Constraint::Length(8), // Bottom content (trades + orders)
            Constraint::Length(1), // Keybindings help
        ])
        .split(area);

    // Tab bar
    tab_bar::render(frame, main_layout[0], app);

    // Status bar
    status_bar::render(frame, main_layout[1], app);

    // Ticker header
    render_ticker_header(frame, main_layout[2], app, symbol);

    // Main content: Order Book | Chart
    let main_content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_layout[3]);

    render_orderbook(frame, main_content[0], app, symbol);
    render_chart(frame, main_content[1], app, symbol);

    // Bottom content: Trades | Orders
    let bottom_content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_layout[4]);

    render_trades(frame, bottom_content[0], app, symbol);
    render_orders(frame, bottom_content[1], app, symbol);

    // Keybindings help
    render_keybindings(frame, main_layout[5], app);
}

/// Renders the ticker header with price info.
fn render_ticker_header(frame: &mut Frame, area: Rect, app: &App, symbol: &str) {
    let ticker = app.tickers.get(symbol);

    let content = if let Some(t) = ticker {
        let change_color = if t.change >= Decimal::ZERO {
            Color::Green
        } else {
            Color::Red
        };
        let arrow = if t.change >= Decimal::ZERO {
            "▲"
        } else {
            "▼"
        };

        Line::from(vec![
            Span::styled(
                format!(" {} ", symbol),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(arrow, Style::default().fg(change_color)),
            Span::styled(
                format!(" {:.2} ", t.last),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Bid: "),
            Span::styled(format!("{:.2} ", t.bid), Style::default().fg(Color::Green)),
            Span::raw("Ask: "),
            Span::styled(format!("{:.2} ", t.ask), Style::default().fg(Color::Red)),
            Span::styled(
                format!("{:+.2}%", t.change_pct),
                Style::default().fg(change_color),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!(" {} ", symbol),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(" -- ", Style::default().fg(Color::DarkGray)),
        ])
    };

    let para = Paragraph::new(content).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(para, area);
}

/// Renders the order book with history.
fn render_orderbook(frame: &mut Frame, area: Rect, app: &App, symbol: &str) {
    let is_focused = app.focus == Focus::OrderBook;
    let is_stale = app.orderbooks.get(symbol).is_some_and(|ob| ob.is_stale);
    let title = if is_stale {
        " Order Book [STALE] "
    } else {
        " Order Book "
    };
    let border_style = if is_stale {
        Style::default().fg(Color::Yellow)
    } else if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split the order book area: depth view on top, history below
    let orderbook_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(inner);

    render_orderbook_depth(frame, orderbook_layout[0], app, symbol);
    render_orderbook_history(frame, orderbook_layout[1], app, symbol);
}

/// Renders the order book depth (bids/asks).
fn render_orderbook_depth(frame: &mut Frame, area: Rect, app: &App, symbol: &str) {
    let orderbook = app.orderbooks.get(symbol);

    let mut lines: Vec<Line> = Vec::new();

    // Calculate how many levels to show per side
    // Reserve 3 lines for: ASK header, spread, BID header
    let available_height = area.height.saturating_sub(3) as usize;
    let levels_per_side = (available_height / 2).clamp(1, 10);

    // ASK header
    lines.push(Line::from(Span::styled(
        "ASK",
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    )));

    if let Some(ob) = orderbook {
        // Show asks (reversed so lowest ask is at bottom, closest to spread)
        let max_qty = ob
            .asks
            .iter()
            .take(levels_per_side)
            .map(|a| a.qty)
            .max()
            .unwrap_or(Decimal::ONE);

        for ask in ob.asks.iter().take(levels_per_side).rev() {
            let bar_len = ((ask.qty / max_qty) * Decimal::from(15))
                .to_string()
                .parse::<usize>()
                .unwrap_or(1);
            let bar = "▒".repeat(bar_len.min(15));

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:>12.2} ", ask.price),
                    Style::default().fg(Color::Red),
                ),
                Span::raw(format!("{:>10.4} ", ask.qty)),
                Span::styled(bar, Style::default().fg(Color::Red)),
            ]));
        }

        // Spread line
        if let (Some(best_bid), Some(best_ask)) = (ob.bids.first(), ob.asks.first()) {
            let spread = best_ask.price - best_bid.price;
            let spread_pct = (spread / best_bid.price) * Decimal::from(100);
            lines.push(Line::from(Span::styled(
                format!("─── Spread: {:.2} ({:.3}%) ───", spread, spread_pct),
                Style::default().fg(Color::DarkGray),
            )));
        }

        // BID header
        lines.push(Line::from(Span::styled(
            "BID",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));

        let max_qty = ob
            .bids
            .iter()
            .take(levels_per_side)
            .map(|b| b.qty)
            .max()
            .unwrap_or(Decimal::ONE);

        for bid in ob.bids.iter().take(levels_per_side) {
            let bar_len = ((bid.qty / max_qty) * Decimal::from(15))
                .to_string()
                .parse::<usize>()
                .unwrap_or(1);
            let bar = "▒".repeat(bar_len.min(15));

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:>12.2} ", bid.price),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(format!("{:>10.4} ", bid.qty)),
                Span::styled(bar, Style::default().fg(Color::Green)),
            ]));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No data",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

/// Extracts time portion (HH:MM:SS) from an RFC3339 timestamp.
fn extract_time(timestamp: &str) -> &str {
    // RFC3339 format: "2024-01-15T12:34:56.789Z" or "2024-01-15T12:34:56.789000Z"
    // Extract the time portion after 'T' and before '.' or 'Z'
    if let Some(t_pos) = timestamp.find('T') {
        let after_t = &timestamp[t_pos + 1..];
        // Find the end of HH:MM:SS (before milliseconds or timezone)
        let end = after_t
            .find('.')
            .or_else(|| after_t.find('Z'))
            .unwrap_or(after_t.len());
        &after_t[..end.min(8)] // Cap at 8 chars for HH:MM:SS
    } else {
        timestamp
    }
}

/// Renders the order book history table.
fn render_orderbook_history(frame: &mut Frame, area: Rect, app: &App, symbol: &str) {
    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(Span::styled(
        "History",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));

    // Column headers
    lines.push(Line::from(Span::styled(
        format!(
            "{:>8}  {:>12}  {:>12}  {:>8}",
            "Time", "Bid", "Ask", "Spread"
        ),
        Style::default().fg(Color::DarkGray),
    )));

    if let Some(ob) = app.orderbooks.get(symbol) {
        let max_rows = area.height.saturating_sub(2) as usize;

        // Iterate in reverse to show most recent first
        let history_iter: Vec<_> = ob.history.iter().rev().take(max_rows).collect();

        for (i, snapshot) in history_iter.iter().enumerate() {
            // Extract time from RFC3339 timestamp
            let time_str = extract_time(&snapshot.timestamp);

            // Calculate spread delta from next (previous in time) snapshot if available
            let spread_delta = if i + 1 < history_iter.len() {
                let prev_spread = history_iter[i + 1].spread;
                let delta = snapshot.spread - prev_spread;
                if delta > Decimal::ZERO {
                    format!("+{:.2}", delta)
                } else if delta < Decimal::ZERO {
                    format!("{:.2}", delta)
                } else {
                    "=".to_string()
                }
            } else {
                "-".to_string()
            };

            let spread_color = if spread_delta.starts_with('+') {
                Color::Red // Spread widening is bad
            } else if spread_delta.starts_with('-') {
                Color::Green // Spread tightening is good
            } else {
                Color::DarkGray
            };

            lines.push(Line::from(vec![
                Span::raw(format!("{:>8}  ", time_str)),
                Span::styled(
                    format!("{:>12.2}  ", snapshot.best_bid),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("{:>12.2}  ", snapshot.best_ask),
                    Style::default().fg(Color::Red),
                ),
                Span::styled(
                    format!("{:>8}", spread_delta),
                    Style::default().fg(spread_color),
                ),
            ]));
        }

        if ob.history.is_empty() {
            lines.push(Line::from(Span::styled(
                "No history yet",
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No data",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

/// Renders the chart panel.
fn render_chart(frame: &mut Frame, area: Rect, app: &App, symbol: &str) {
    let is_focused = app.focus == Focus::Chart;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let chart_type_label = match app.chart_type {
        ChartType::Candle => "Candle",
        ChartType::Line => "Line",
    };

    let title = format!(
        " Chart [{}] {} ",
        chart_type_label,
        app.chart_timeframe.label()
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let candles = app.candles.get(symbol);

    if let Some(candle_data) = candles {
        if candle_data.is_empty() {
            let para = Paragraph::new("No candle data").style(Style::default().fg(Color::DarkGray));
            frame.render_widget(para, inner);
        } else {
            // Simple ASCII chart visualization
            let mut lines: Vec<Line> = Vec::new();

            // Find price range
            let (min_price, max_price) = candle_data
                .iter()
                .fold((Decimal::MAX, Decimal::ZERO), |(min, max), c| {
                    (min.min(c.low), max.max(c.high))
                });

            let price_range = max_price - min_price;
            let height = inner.height.saturating_sub(2) as usize;

            if price_range > Decimal::ZERO && height > 0 {
                // Build chart rows
                for row in 0..height {
                    let price_level =
                        max_price - (price_range * Decimal::from(row) / Decimal::from(height));

                    let mut row_chars: Vec<Span> = Vec::new();
                    row_chars.push(Span::raw(format!("{:>10.2} │", price_level)));

                    for candle in candle_data.iter().rev().take(inner.width as usize - 12) {
                        let is_bullish = candle.close >= candle.open;
                        let color = if is_bullish { Color::Green } else { Color::Red };

                        let body_top = candle.open.max(candle.close);
                        let body_bottom = candle.open.min(candle.close);

                        let char = if price_level <= candle.high && price_level >= body_top {
                            "│" // Upper wick
                        } else if price_level < body_top && price_level > body_bottom {
                            "█" // Body
                        } else if price_level <= body_bottom && price_level >= candle.low {
                            "│" // Lower wick
                        } else {
                            " "
                        };

                        row_chars.push(Span::styled(char, Style::default().fg(color)));
                    }

                    lines.push(Line::from(row_chars));
                }

                // Timeframe selector
                let tf_line = Line::from(vec![
                    Span::raw("           "),
                    Span::styled(
                        " 1m ",
                        if app.chart_timeframe == crate::tui::app::Timeframe::M1 {
                            Style::default().bg(Color::Cyan).fg(Color::Black)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(
                        " 5m ",
                        if app.chart_timeframe == crate::tui::app::Timeframe::M5 {
                            Style::default().bg(Color::Cyan).fg(Color::Black)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(
                        " 15m ",
                        if app.chart_timeframe == crate::tui::app::Timeframe::M15 {
                            Style::default().bg(Color::Cyan).fg(Color::Black)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(
                        " 1h ",
                        if app.chart_timeframe == crate::tui::app::Timeframe::H1 {
                            Style::default().bg(Color::Cyan).fg(Color::Black)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(
                        " 4h ",
                        if app.chart_timeframe == crate::tui::app::Timeframe::H4 {
                            Style::default().bg(Color::Cyan).fg(Color::Black)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(
                        " 1d ",
                        if app.chart_timeframe == crate::tui::app::Timeframe::D1 {
                            Style::default().bg(Color::Cyan).fg(Color::Black)
                        } else {
                            Style::default()
                        },
                    ),
                ]);
                lines.push(tf_line);
            }

            let para = Paragraph::new(lines);
            frame.render_widget(para, inner);
        }
    } else {
        let para = Paragraph::new("No data").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(para, inner);
    }
}

/// Renders the trades panel with BUY and SELL columns.
fn render_trades(frame: &mut Frame, area: Rect, app: &App, symbol: &str) {
    let is_focused = app.focus == Focus::Trades;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Trades ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into two columns: BUY | SELL
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let trades = app.recent_trades.get(symbol);

    // Separate trades by side
    let (buy_trades, sell_trades): (Vec<_>, Vec<_>) = if let Some(trade_list) = trades {
        trade_list
            .iter()
            .rev()
            .partition(|t| t.side.to_uppercase() == "BUY")
    } else {
        (Vec::new(), Vec::new())
    };

    // Render BUY column
    render_trades_column(frame, columns[0], "BUY", Color::Green, &buy_trades);

    // Render SELL column
    render_trades_column(frame, columns[1], "SELL", Color::Red, &sell_trades);
}

/// Renders a single trades column (BUY or SELL).
fn render_trades_column(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    color: Color,
    trades: &[&crate::models::trade::TradeData],
) {
    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(vec![Span::styled(
        format!(
            " {:^width$}",
            title,
            width = area.width.saturating_sub(2) as usize
        ),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )]));

    // Column headers
    lines.push(Line::from(vec![Span::styled(
        format!(" {:>10}  {:>8}", "Price", "Volume"),
        Style::default().fg(Color::DarkGray),
    )]));

    // Trade rows
    let max_rows = area.height.saturating_sub(2) as usize;
    for trade in trades.iter().take(max_rows) {
        lines.push(Line::from(vec![Span::styled(
            format!(" {:>10.2}  {:>8.4}", trade.price, trade.qty),
            Style::default().fg(color),
        )]));
    }

    // Fill empty rows if needed
    if trades.is_empty() {
        lines.push(Line::from(Span::styled(
            " No trades",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

/// Renders the orders panel.
fn render_orders(frame: &mut Frame, area: Rect, app: &App, symbol: &str) {
    let is_focused = app.focus == Focus::Orders;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let view_label = match app.orders_view {
        OrdersView::Open => "Open",
        OrdersView::Executed => "Executed",
    };

    let title = format!(" Orders [{}] ", view_label);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let orders = match app.orders_view {
        OrdersView::Open => app.open_orders.get(symbol),
        OrdersView::Executed => None, // VecDeque vs Vec, handle separately
    };

    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(Span::styled(
        format!(
            "{:<12} {:<6} {:<8} {:>12} {:>10}",
            "ID", "Side", "Type", "Price", "Qty"
        ),
        Style::default().add_modifier(Modifier::BOLD),
    )));

    if let Some(order_list) = orders {
        for order in order_list
            .iter()
            .take(inner.height.saturating_sub(1) as usize)
        {
            let side_color = if order.side.to_uppercase() == "BUY" {
                Color::Green
            } else {
                Color::Red
            };

            let id_short = if order.order_id.len() > 10 {
                format!("{}...", &order.order_id[..10])
            } else {
                order.order_id.clone()
            };

            let price = order.limit_price.unwrap_or(Decimal::ZERO);

            lines.push(Line::from(vec![
                Span::raw(format!("{:<12} ", id_short)),
                Span::styled(
                    format!("{:<6} ", order.side.to_uppercase()),
                    Style::default().fg(side_color),
                ),
                Span::raw(format!("{:<8} ", order.order_type)),
                Span::raw(format!("{:>12.2} ", price)),
                Span::raw(format!("{:>10.4}", order.order_qty)),
            ]));
        }
    }

    if app.orders_view == OrdersView::Executed
        && let Some(executed) = app.executed_orders.get(symbol)
    {
        for order in executed
            .iter()
            .rev()
            .take(inner.height.saturating_sub(1) as usize)
        {
            let side_color = if order.side.to_uppercase() == "BUY" {
                Color::Green
            } else {
                Color::Red
            };

            let id_short = if order.order_id.len() > 10 {
                format!("{}...", &order.order_id[..10])
            } else {
                order.order_id.clone()
            };

            let price = order.avg_price.unwrap_or(Decimal::ZERO);

            lines.push(Line::from(vec![
                Span::raw(format!("{:<12} ", id_short)),
                Span::styled(
                    format!("{:<6} ", order.side.to_uppercase()),
                    Style::default().fg(side_color),
                ),
                Span::raw(format!("{:<8} ", order.order_type)),
                Span::raw(format!("{:>12.2} ", price)),
                Span::raw(format!("{:>10.4}", order.order_qty)),
            ]));
        }
    }

    if lines.len() == 1 {
        lines.push(Line::from(Span::styled(
            "No orders",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

/// Renders the keybindings help line.
fn render_keybindings(frame: &mut Frame, area: Rect, _app: &App) {
    let help = "[n]ew order [c]ancel [e]dit [g]chart type [o]orders view [1-6]timeframe [Tab]switch tab [?]help [q]quit";

    let para = Paragraph::new(help).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(para, area);
}
