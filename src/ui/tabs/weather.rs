use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App, area: Rect, is_focused: bool) {
    let border_color = if is_focused { Color::White } else { Color::Cyan };

    if app.weather.loading {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Weather ")
            .border_style(Style::default().fg(border_color));
        let p = Paragraph::new("Fetching weather...")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
        return;
    }

    if let Some(err) = &app.weather.error {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Weather ")
            .border_style(Style::default().fg(border_color));
        let text = vec![
            Line::from(Span::styled(
                "Error",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::raw(err.clone())),
            Line::from(""),
            Line::from(Span::styled("Press [r] to retry", Style::default().fg(Color::DarkGray))),
        ];
        let p = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    if let Some(data) = &app.weather.data {
        let title = format!(" Weather — {} ", data.location);
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));
        let inner = outer_block.inner(area);
        f.render_widget(outer_block, area);

        let n = data.forecast.len().min(7);
        if n == 0 {
            return;
        }

        let constraints: Vec<Constraint> =
            (0..n).map(|_| Constraint::Ratio(1, n as u32)).collect();
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(inner);

        for (i, day) in data.forecast.iter().take(n).enumerate() {
            let is_today = i == 0;
            let col_area = cols[i];

            let (border_color, title_style) = if is_today {
                (
                    Color::Yellow,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                (Color::DarkGray, Style::default().fg(Color::White))
            };

            let col_title = if is_today {
                format!(" Today ({}) ", &day.date)
            } else {
                format!(" {} ", &day.date)
            };

            let borders = if i == 0 {
                Borders::TOP
            } else {
                Borders::TOP | Borders::LEFT
            };

            let block = Block::default()
                .borders(borders)
                .title(Span::styled(col_title, title_style))
                .border_style(Style::default().fg(border_color));

            let col_inner = block.inner(col_area);
            f.render_widget(block, col_area);

            let mut lines: Vec<Line> = vec![];

            if is_today {
                lines.push(Line::from(Span::styled(
                    format!("{:.0}°F  {}", data.temp_f, data.description),
                    Style::default()
                        .fg(temp_color(data.temp_f))
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(Span::styled(
                    format!(
                        "Feels {:.0}°  Hum {}%  Wind {:.0}mph",
                        data.feels_like_f, data.humidity, data.wind_mph
                    ),
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
            }

            lines.push(Line::from(Span::styled(
                day.icon,
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(Span::styled(
                format!("Hi {:.0}° / Lo {:.0}°", day.high_f, day.low_f),
                Style::default().fg(temp_color(day.high_f)),
            )));
            lines.push(Line::from(Span::styled(
                day.description.clone(),
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Precip {:.2}\"", day.precip_in),
                Style::default().fg(Color::Cyan),
            )));
            lines.push(Line::from(Span::styled(
                format!("Wind  {:.0} mph", day.wind_max_mph),
                Style::default().fg(Color::DarkGray),
            )));

            let p = Paragraph::new(lines).wrap(Wrap { trim: false });
            f.render_widget(p, col_inner);
        }
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Weather ")
            .border_style(Style::default().fg(border_color));
        let p = Paragraph::new("Press [r] to load weather.")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
    }
}

fn temp_color(temp: f64) -> Color {
    if temp < 32.0 {
        Color::Cyan
    } else if temp < 50.0 {
        Color::Blue
    } else if temp < 70.0 {
        Color::Green
    } else if temp < 85.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}
