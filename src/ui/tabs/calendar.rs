use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect, is_focused: bool) {
    let border_color = if is_focused { Color::White } else { Color::Magenta };

    // OAuth flow in progress
    if let Some(url) = &app.calendar_auth_url {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Google Calendar ")
            .border_style(Style::default().fg(border_color));
        let text = vec![
            Line::from(Span::styled(
                "Waiting for browser authorization…",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Your browser should have opened. If not, visit:",
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(url.clone(), Style::default().fg(Color::Cyan))),
        ];
        let p = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    if app.calendar.loading {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Google Calendar ")
            .border_style(Style::default().fg(border_color));
        let p = Paragraph::new("Loading calendar…")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
        return;
    }

    if let Some(err) = &app.calendar.error {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Google Calendar ")
            .border_style(Style::default().fg(border_color));
        let mut text = vec![
            Line::from(Span::styled(
                "Error",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::raw(err.clone())),
        ];

        if err.contains("not set in .env") {
            text.extend([
                Line::from(""),
                Line::from(Span::styled(
                    "Setup:",
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )),
                Line::from("  1. console.cloud.google.com → enable Google Calendar API"),
                Line::from("  2. OAuth consent screen → Desktop app credentials"),
                Line::from("  3. Set in .env:"),
                Line::from("       GOOGLE_CLIENT_ID=…"),
                Line::from("       GOOGLE_CLIENT_SECRET=…"),
                Line::from("       GOOGLE_CALENDAR_ID=primary"),
                Line::from("  4. Press [r] — browser will open for one-time authorization"),
            ]);
        } else {
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Press [r] to retry",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let p = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    if let Some(events) = &app.calendar.data {
        let title = format!(
            " Google Calendar  [d] toggle days  [j/k] scroll ({} events) ",
            events.len()
        );
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));
        let inner = outer_block.inner(area);
        f.render_widget(outer_block, area);

        let today = Local::now().date_naive();

        let enabled_days: Vec<(usize, chrono::NaiveDate)> = (0..7usize)
            .filter(|&i| app.calendar_days_enabled[i])
            .map(|i| (i, today + chrono::Duration::days(i as i64)))
            .collect();

        if enabled_days.is_empty() {
            let p = Paragraph::new("No days selected. Press [d] to toggle days.")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(p, inner);
        } else {
            let n = enabled_days.len();
            let constraints: Vec<Constraint> =
                (0..n).map(|_| Constraint::Ratio(1, n as u32)).collect();
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .split(inner);

            for (col_idx, (day_idx, date)) in enabled_days.iter().enumerate() {
                let col_area = cols[col_idx];
                let is_today = *day_idx == 0;
                let date_str = date.format("%Y-%m-%d").to_string();

                let day_events: Vec<_> = events
                    .iter()
                    .filter(|e| e.start.starts_with(&date_str))
                    .collect();

                let col_title = if is_today {
                    format!(" Today {} ", date.format("%a %b %d"))
                } else {
                    format!(" {} ", date.format("%a %b %d"))
                };

                let (bdr_color, title_style) = if is_today {
                    (
                        Color::Yellow,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    (Color::DarkGray, Style::default().fg(Color::White))
                };

                let borders = if col_idx == 0 {
                    Borders::TOP
                } else {
                    Borders::TOP | Borders::LEFT
                };

                let block = Block::default()
                    .borders(borders)
                    .title(Span::styled(col_title, title_style))
                    .border_style(Style::default().fg(bdr_color));

                let col_inner = block.inner(col_area);
                f.render_widget(block, col_area);

                let mut lines: Vec<Line> = vec![];

                if day_events.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "No events",
                        Style::default().fg(Color::DarkGray),
                    )));
                } else {
                    for e in &day_events {
                        let time_str = if e.start.len() > 10 {
                            e.start[11..].trim_end_matches(':').to_string()
                        } else {
                            "All day".to_string()
                        };
                        lines.push(Line::from(Span::styled(
                            time_str,
                            Style::default().fg(Color::DarkGray),
                        )));
                        lines.push(Line::from(Span::styled(
                            e.title.clone(),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        )));
                        if let Some(loc) = &e.location {
                            lines.push(Line::from(Span::styled(
                                loc.clone(),
                                Style::default().fg(Color::DarkGray),
                            )));
                        }
                        lines.push(Line::from(""));
                    }
                }

                let p = Paragraph::new(lines)
                    .scroll((app.calendar_scroll, 0))
                    .wrap(Wrap { trim: false });
                f.render_widget(p, col_inner);
            }
        }
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Google Calendar ")
            .border_style(Style::default().fg(border_color));
        let p = Paragraph::new("Press [r] to load calendar.")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
    }

    if app.calendar_day_picker_open {
        render_day_picker(f, app, area);
    }
}

fn render_day_picker(f: &mut Frame, app: &App, area: Rect) {
    let today = Local::now().date_naive();

    let height = 11u16; // 7 items + borders
    let width = 36u16;
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let popup = Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    };

    f.render_widget(Clear, popup);

    let items: Vec<ListItem> = (0..7usize)
        .map(|i| {
            let date = today + chrono::Duration::days(i as i64);
            let enabled = app.calendar_days_enabled[i];
            let checkbox = if enabled { "[x]" } else { "[ ]" };
            let check_color = if enabled { Color::Green } else { Color::DarkGray };
            let label = if i == 0 {
                format!("Today  {}", date.format("%a %b %d"))
            } else {
                date.format("%a %b %d").to_string()
            };
            let label_color = if i == app.calendar_day_cursor {
                Color::Yellow
            } else if enabled {
                Color::White
            } else {
                Color::DarkGray
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {checkbox} "), Style::default().fg(check_color)),
                Span::styled(label, Style::default().fg(label_color)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Days  [Space] toggle  [d/Esc] close ")
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = ListState::default();
    state.select(Some(app.calendar_day_cursor));
    f.render_stateful_widget(list, popup, &mut state);
}
