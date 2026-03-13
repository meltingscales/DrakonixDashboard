mod tabs;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
    Frame,
};

use crate::app::{App, Tab};
use crate::tiling::{Side, SplitDir, Tile};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Split: tile area on top, tab bar on bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    render_tile(f, app, &app.tiles.root, chunks[0], &app.tiles.focus_path.0, &[]);
    render_tab_bar(f, app, chunks[1]);

    if app.show_help {
        render_help_overlay(f, area);
    }
}

fn render_tile(
    f: &mut Frame,
    app: &App,
    tile: &Tile,
    area: Rect,
    focus_path: &[Side],
    current_path: &[Side],
) {
    match tile {
        Tile::Leaf(tab) => {
            let is_focused = !app.tiles.is_single() && focus_path == current_path;
            render_leaf(f, app, *tab, area, is_focused);
        }
        Tile::Split { dir, ratio, left, right } => {
            let (left_area, right_area) = split_rect(area, *dir, *ratio);

            let mut left_path = current_path.to_vec();
            left_path.push(Side::Left);
            let mut right_path = current_path.to_vec();
            right_path.push(Side::Right);

            render_tile(f, app, left, left_area, focus_path, &left_path);
            render_tile(f, app, right, right_area, focus_path, &right_path);
        }
    }
}

fn split_rect(area: Rect, dir: SplitDir, ratio: f32) -> (Rect, Rect) {
    let pct = (ratio * 100.0) as u16;
    let rest = 100u16.saturating_sub(pct);
    let direction = match dir {
        SplitDir::Horizontal => Direction::Horizontal,
        SplitDir::Vertical => Direction::Vertical,
    };
    let chunks = Layout::default()
        .direction(direction)
        .constraints([Constraint::Percentage(pct), Constraint::Percentage(rest)])
        .split(area);
    (chunks[0], chunks[1])
}

fn render_leaf(f: &mut Frame, app: &App, tab: Tab, area: Rect, is_focused: bool) {
    match tab {
        Tab::Weather => tabs::weather::render(f, app, area, is_focused),
        Tab::Calendar => tabs::calendar::render(f, app, area, is_focused),
        Tab::Rss => tabs::rss::render(f, app, area, is_focused),
    }
}

fn render_help_overlay(f: &mut Frame, area: Rect) {
    let width = 52u16;
    let height = 30u16;
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let popup = Rect { x, y, width: width.min(area.width), height: height.min(area.height) };

    f.render_widget(Clear, popup);

    let rows: &[(&str, &str)] = &[
        ("Navigation", ""),
        ("  1 / 2 / 3", "Switch focused pane to tab"),
        ("  ← / →", "Cycle tab in focused pane"),
        ("  Tab", "Focus next pane"),
        ("  Shift+Tab", "Focus previous pane"),
        ("", ""),
        ("Tiling", ""),
        ("  |  or  \\", "Split pane side-by-side"),
        ("  -", "Split pane top / bottom"),
        ("  x", "Close focused pane"),
        ("", ""),
        ("Within RSS tab", ""),
        ("  j / ↓", "Next item"),
        ("  k / ↑", "Previous item"),
        ("  Enter", "Open article in browser"),
        ("  f", "Toggle feed sources modal"),
        ("", ""),
        ("Within Calendar tab", ""),
        ("  j / ↓", "Scroll down"),
        ("  k / ↑", "Scroll up"),
        ("  d", "Toggle day columns modal"),
        ("", ""),
        ("General", ""),
        ("  r", "Refresh focused pane"),
        ("  ?", "Toggle this help"),
        ("  q  or  Esc", "Quit (or close help)"),
    ];

    let lines: Vec<Line> = rows
        .iter()
        .map(|(key, desc)| {
            if desc.is_empty() && !key.is_empty() {
                // Section header
                Line::from(Span::styled(
                    *key,
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ))
            } else if key.is_empty() {
                Line::from("")
            } else {
                Line::from(vec![
                    Span::styled(format!("{:<22}", key), Style::default().fg(Color::Cyan)),
                    Span::styled(*desc, Style::default().fg(Color::White)),
                ])
            }
        })
        .collect();

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Controls  [? or Esc to close] ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(p, popup);
}

fn render_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_tab();
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let label = format!(" {} ", tab.label());
            let key = format!("[{}] ", i + 1);
            if *tab == focused {
                Line::from(vec![
                    Span::styled(key, Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        label,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(key, Style::default().fg(Color::DarkGray)),
                    Span::styled(label, Style::default().fg(Color::White)),
                ])
            }
        })
        .collect();

    let hint = if app.tiles.is_single() {
        " [|] split  [-] split  [?] help "
    } else {
        " [Tab] next pane  [x] close  [|/-] split  [?] help "
    };

    let tabs_widget = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(hint)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .select(focused.index())
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" | ", Style::default().fg(Color::DarkGray)));

    f.render_widget(tabs_widget, area);
}
