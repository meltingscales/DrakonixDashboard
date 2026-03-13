use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect, is_focused: bool) {
    let border_color = if is_focused { Color::White } else { Color::Green };
    if app.rss.loading {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" RSS ")
            .border_style(Style::default().fg(border_color));
        let p = Paragraph::new("Fetching RSS feeds...")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
        return;
    }

    if let Some(err) = &app.rss.error {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" RSS ")
            .border_style(Style::default().fg(if is_focused { Color::White } else { Color::Red }));
        let text = vec![
            Line::from(Span::styled("Error", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(Span::raw(err.clone())),
            Line::from(""),
            Line::from(Span::styled("Press [r] to retry", Style::default().fg(Color::DarkGray))),
        ];
        let p = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    if let Some(items) = &app.rss.data {
        if items.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" RSS ")
                .border_style(Style::default().fg(border_color));
            let p = Paragraph::new("No RSS feeds configured.\nAdd RSS_FEEDS to .env and press [r].")
                .block(block)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(p, area);
            return;
        }

        // Split: list on left, detail on right
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        // List of items
        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == app.rss_selected {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };
                let source_style = if i == app.rss_selected {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let line = Line::from(vec![
                    Span::styled(item.title.clone(), style),
                ]);
                let source_line = Line::from(vec![
                    Span::styled(
                        format!("  {} {}", item.source, item.published.as_deref().unwrap_or("")),
                        source_style,
                    ),
                ]);

                ListItem::new(vec![line, source_line])
            })
            .collect();

        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" RSS ({} items) ", items.len()))
                    .border_style(Style::default().fg(border_color)),
            )
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Green));

        let mut list_state = ListState::default();
        list_state.select(Some(app.rss_selected));
        f.render_stateful_widget(list, chunks[0], &mut list_state);

        // Detail pane for selected item
        let selected = &items[app.rss_selected];
        let mut detail_lines = vec![
            Line::from(Span::styled(
                selected.title.clone(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("Source: {}", selected.source),
                Style::default().fg(Color::Green),
            )),
        ];
        if let Some(pub_date) = &selected.published {
            detail_lines.push(Line::from(Span::styled(
                format!("Published: {}", pub_date),
                Style::default().fg(Color::DarkGray),
            )));
        }
        if let Some(link) = &selected.link {
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled(
                link.clone(),
                Style::default().fg(Color::Cyan),
            )));
        }
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(Span::styled(
            "↑/↓ or j/k to navigate  •  Enter to open in browser",
            Style::default().fg(Color::DarkGray),
        )));

        let detail = Paragraph::new(detail_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Detail ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(detail, chunks[1]);
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" RSS ")
            .border_style(Style::default().fg(border_color));
        let p = Paragraph::new("Press [r] to load RSS feeds.")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
    }

    if app.rss_feed_picker_open {
        render_feed_picker(f, app, area);
    }
}

fn render_feed_picker(f: &mut Frame, app: &App, area: Rect) {
    let feeds = &app.config.rss_feeds;

    if feeds.is_empty() {
        let width = 44u16;
        let height = 7u16;
        let x = area.x + area.width.saturating_sub(width) / 2;
        let y = area.y + area.height.saturating_sub(height) / 2;
        let popup = Rect { x, y, width: width.min(area.width), height: height.min(area.height) };
        f.render_widget(Clear, popup);
        let text = vec![
            Line::from(Span::styled("No feeds configured.", Style::default().fg(Color::DarkGray))),
            Line::from(""),
            Line::from("Add to .env:"),
            Line::from(Span::styled("  RSS_FEEDS=url1,url2,...", Style::default().fg(Color::Cyan))),
            Line::from(""),
            Line::from(Span::styled("Then press [r] to load.", Style::default().fg(Color::DarkGray))),
        ];
        let p = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Feed Sources  [f/Esc] close ")
                    .border_style(Style::default().fg(Color::Green)),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(p, popup);
        return;
    }

    let height = (feeds.len() as u16 + 4).min(area.height.saturating_sub(2));
    let width = feeds
        .iter()
        .map(|u| u.len() as u16 + 8) // room for "[x] " prefix and borders
        .max()
        .unwrap_or(40)
        .clamp(36, area.width.saturating_sub(4));

    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let popup = Rect { x, y, width, height };

    f.render_widget(Clear, popup);

    let items: Vec<ListItem> = feeds
        .iter()
        .zip(&app.rss_feed_enabled)
        .enumerate()
        .map(|(i, (url, &enabled))| {
            let checkbox = if enabled { "[x]" } else { "[ ]" };
            let check_color = if enabled { Color::Green } else { Color::DarkGray };
            let label_color = if i == app.rss_feed_cursor {
                Color::Yellow
            } else if enabled {
                Color::White
            } else {
                Color::DarkGray
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {checkbox} "), Style::default().fg(check_color)),
                Span::styled(url.as_str(), Style::default().fg(label_color)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Feed Sources  [Space] toggle  [f/Esc] close ")
                .border_style(Style::default().fg(Color::Green)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = ListState::default();
    state.select(Some(app.rss_feed_cursor));
    f.render_stateful_widget(list, popup, &mut state);
}

