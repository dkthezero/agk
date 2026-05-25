#![allow(dead_code)]

use crate::domain::telemetry::AnalyticsConfig;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

/// Render the Telemetry analytics dashboard.
pub fn render(frame: &mut Frame, area: Rect, config: &AnalyticsConfig, selected: usize) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Telemetry — Skill Usage Analytics");

    if !config.settings.enabled {
        let help = vec![Line::from(vec![Span::styled(
            "Telemetry is disabled.",
            Style::default().fg(Color::DarkGray),
        )])];
        frame.render_widget(Paragraph::new(help).block(block), area);
        return;
    }

    if config.skills.is_empty() {
        let help = vec![Line::from(vec![
            Span::styled(
                "No usage data yet. Install skills and wait for the background scanner to collect data.",
                Style::default().fg(Color::DarkGray),
            ),
        ])];
        frame.render_widget(Paragraph::new(help).block(block), area);
        return;
    }

    // Build table rows
    let mut rows: Vec<Row> = Vec::new();
    let mut items: Vec<(
        String,
        u64,
        String,
        String,
        bool, // is stale
    )> = Vec::new();

    let now = chrono::Utc::now();
    let stale_threshold = chrono::Duration::days(30);

    for (name, analytics) in &config.skills {
        let is_stale = analytics
            .last_used
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| {
                now.signed_duration_since(chrono::DateTime::<chrono::Utc>::from(dt))
                    > stale_threshold
            })
            .unwrap_or(true);

        let providers = analytics.providers().join(", ");
        let last_used = analytics
            .last_used
            .clone()
            .unwrap_or_else(|| "never".to_string());

        items.push((
            name.clone(),
            analytics.total_invocations,
            last_used,
            providers,
            is_stale,
        ));
    }

    // Sort by total_invocations descending
    items.sort_by(|a, b| b.1.cmp(&a.1));

    for (i, (name, invocations, last_used, providers, is_stale)) in items.iter().enumerate() {
        let style = if *is_stale {
            Style::default().fg(Color::DarkGray)
        } else if i == selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        rows.push(
            Row::new(vec![
                Cell::from(name.as_str()).style(style),
                Cell::from(format!("{}", invocations)).style(style),
                Cell::from(last_used.as_str()).style(style),
                Cell::from(providers.as_str()).style(style),
            ])
            .style(style),
        );
    }

    let header = Row::new(vec![
        Cell::from("Skill Name").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Invocations").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Last Used").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Providers").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let widths = [
        ratatui::layout::Constraint::Percentage(35),
        ratatui::layout::Constraint::Percentage(15),
        ratatui::layout::Constraint::Percentage(25),
        ratatui::layout::Constraint::Percentage(25),
    ];

    let table = Table::new(rows, widths)
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

    let mut items: Vec<(String, u64, String, String, bool, String)> = config
        .skills
        .iter()
        .map(|(name, analytics)| {
            let is_stale = analytics
                .last_used
                .as_ref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| {
                    let now = chrono::Utc::now();
                    now.signed_duration_since(chrono::DateTime::<chrono::Utc>::from(dt))
                        > chrono::Duration::days(30)
                })
                .unwrap_or(true);
            (
                name.clone(),
                analytics.total_invocations,
                analytics
                    .last_used
                    .clone()
                    .unwrap_or_else(|| "never".to_string()),
                analytics.providers().join(", "),
                is_stale,
                analytics.provider_breakdown(),
            )
        })
        .collect();

    items.sort_by(|a, b| b.1.cmp(&a.1));

    let Some((name, invocations, last_used, providers, is_stale, breakdown)) = items.get(selected)
    else {
        frame.render_widget(Paragraph::new("No data selected.").block(block), area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Skill: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(name.as_str()),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "Total invocations: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", invocations)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Last used: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(last_used.as_str()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Providers: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(providers.as_str()),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "Per-provider: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(breakdown.as_str()),
    ]));

    if *is_stale {
        lines.push(Line::from(vec![Span::styled(
            "[STALE] No usage in the last 30 days",
            Style::default().fg(Color::DarkGray),
        )]));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}
