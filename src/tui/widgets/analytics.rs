#![allow(dead_code)]

use crate::domain::telemetry::AnalyticsConfig;
use std::cmp::Reverse;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

/// A row in the combined analytics table.
struct AnalyticsRow {
    category: String,
    name: String,
    count: u64,
    last_used: String,
    providers: String,
    is_stale: bool,
    breakdown: String,
}

fn is_stale(timestamp: Option<&String>) -> bool {
    let now = chrono::Utc::now();
    timestamp
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| {
            now.signed_duration_since(chrono::DateTime::<chrono::Utc>::from(dt))
                > chrono::Duration::days(30)
        })
        .unwrap_or(true)
}

fn build_rows(config: &AnalyticsConfig) -> Vec<AnalyticsRow> {
    let mut rows: Vec<AnalyticsRow> = Vec::new();

    for (name, a) in &config.skills {
        rows.push(AnalyticsRow {
            category: "skill".into(),
            name: name.clone(),
            count: a.total_invocations,
            last_used: a.last_used.clone().unwrap_or_else(|| "never".into()),
            providers: a.providers().join(", "),
            is_stale: is_stale(a.last_used.as_ref()),
            breakdown: a.provider_breakdown(),
        });
    }

    for (name, a) in &config.templates {
        rows.push(AnalyticsRow {
            category: "template".into(),
            name: name.clone(),
            count: a.selections,
            last_used: a.last_selected.clone().unwrap_or_else(|| "never".into()),
            providers: "N/A".into(),
            is_stale: false,
            breakdown: "N/A".into(),
        });
    }

    for (name, a) in &config.profiles {
        rows.push(AnalyticsRow {
            category: "profile".into(),
            name: name.clone(),
            count: a.launches,
            last_used: a.last_launched.clone().unwrap_or_else(|| "never".into()),
            providers: a.provider.clone().unwrap_or_else(|| "unknown".into()),
            is_stale: is_stale(a.last_launched.as_ref()),
            breakdown: a.provider.clone().unwrap_or_else(|| "unknown".into()),
        });
    }

    rows.sort_by_key(|r| Reverse(r.count));
    rows
}

/// Render the Telemetry analytics dashboard showing skills, templates, and profiles.
pub fn render(frame: &mut Frame, area: Rect, config: &AnalyticsConfig, selected: usize) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Telemetry — Usage Analytics");

    if !config.settings.enabled {
        let help = vec![Line::from(vec![Span::styled(
            "Telemetry is disabled.",
            Style::default().fg(Color::DarkGray),
        )])];
        frame.render_widget(Paragraph::new(help).block(block), area);
        return;
    }

    let rows = build_rows(config);
    if rows.is_empty() {
        let help = vec![Line::from(vec![Span::styled(
            "No usage data yet. Install skills and wait for the background scanner.",
            Style::default().fg(Color::DarkGray),
        )])];
        frame.render_widget(Paragraph::new(help).block(block), area);
        return;
    }

    let table_rows: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let style = if r.is_stale {
                Style::default().fg(Color::DarkGray)
            } else if i == selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Row::new(vec![
                Cell::from(r.category.as_str()).style(style),
                Cell::from(r.name.as_str()).style(style),
                Cell::from(format!("{}", r.count)).style(style),
                Cell::from(r.last_used.as_str()).style(style),
                Cell::from(r.providers.as_str()).style(style),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Category").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Name").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Count").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Last Used").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Providers").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let widths = [
        ratatui::layout::Constraint::Percentage(12),
        ratatui::layout::Constraint::Percentage(28),
        ratatui::layout::Constraint::Percentage(12),
        ratatui::layout::Constraint::Percentage(28),
        ratatui::layout::Constraint::Percentage(20),
    ];

    let table = Table::new(table_rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_widget(table, area);
}

pub fn render_detail(frame: &mut Frame, area: Rect, config: &AnalyticsConfig, selected: usize) {
    let block = Block::default().borders(Borders::ALL).title("Usage Detail");

    if !config.settings.enabled {
        frame.render_widget(
            Paragraph::new("Enable telemetry to collect usage data.\n\nPress Space to toggle.")
                .block(block),
            area,
        );
        return;
    }

    let rows = build_rows(config);
    let Some(r) = rows.get(selected) else {
        frame.render_widget(Paragraph::new("No data selected.").block(block), area);
        return;
    };

    let count_label = match r.category.as_str() {
        "template" => "Selections",
        "profile" => "Launches",
        _ => "Total invocations",
    };
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Category: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(r.category.as_str()),
        ]),
        Line::from(vec![
            Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(r.name.as_str()),
        ]),
        Line::from(vec![
            Span::styled(count_label, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(": {}", r.count)),
        ]),
        Line::from(vec![
            Span::styled("Last used: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(r.last_used.as_str()),
        ]),
        Line::from(vec![
            Span::styled("Providers: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(r.providers.as_str()),
        ]),
    ];

    if r.category == "skill" {
        lines.push(Line::from(vec![
            Span::styled("Per-provider: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(r.breakdown.as_str()),
        ]));
    }

    if r.is_stale {
        lines.push(Line::from(vec![Span::styled(
            "[STALE] No usage in the last 30 days",
            Style::default().fg(Color::DarkGray),
        )]));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}