use crate::app::snapshot::DiscoveredMcp;
use crate::domain::mcp::McpTransport;
use crate::domain::mcp_security::SecuritySeverity;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::McpState;

pub(crate) fn severity_color(sev: &SecuritySeverity) -> Color {
    match sev {
        SecuritySeverity::Low => Color::Blue,
        SecuritySeverity::Medium => Color::Yellow,
        SecuritySeverity::High => Color::Red,
        SecuritySeverity::Critical => Color::Magenta,
    }
}

pub fn render_detail(
    frame: &mut Frame,
    area: Rect,
    state: &McpState,
    active_selected: usize,
    discovered_mcps: &[DiscoveredMcp],
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("MCP Server Detail");

    let items = state.servers_list();
    let registered_count = items.len();
    let discovered_count = discovered_mcps.len();
    let has_discovered = discovered_count > 0;
    // Separator occupies one row if discovered section exists
    let sep_offset = if has_discovered { 1 } else { 0 };

    // Check if selection falls on a discovered MCP
    if has_discovered && active_selected >= registered_count + sep_offset {
        let discovered_idx = active_selected - registered_count - sep_offset;
        if let Some(dm) = discovered_mcps.get(discovered_idx) {
            let lines: Vec<Line> = vec![
                Line::from(vec![
                    Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&dm.name),
                ]),
                Line::from(vec![
                    Span::styled("Vault: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&dm.vault_id),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Description: ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(dm.description.as_deref().unwrap_or("—")),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        "Discovered (not registered)",
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(""),
                Line::from("Press Enter to register this MCP server."),
            ];
            frame.render_widget(
                Paragraph::new(lines)
                    .block(block)
                    .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
    }

    let Some((id, server)) = items.get(active_selected).copied() else {
        frame.render_widget(
            Paragraph::new("No server registered.\n\nUse `agk mcp add` to register a server.")
                .block(block),
            area,
        );
        return;
    };

    let transport = match &server.transport {
        McpTransport::Stdio => "stdio".to_string(),
        McpTransport::Sse { url } => format!("sse: {}", url),
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(id.as_str()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Command: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(server.command.as_str()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Transport: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(transport.as_str()),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "Description: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(server.description.as_deref().unwrap_or("—")),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Tested: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(if server.tested { "Yes" } else { "No" }),
    ]));
    if let Some(ref tested_at) = server.tested_at {
        lines.push(Line::from(vec![
            Span::styled("Tested at: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(tested_at.as_str()),
        ]));
    }

    // Security Assessment section
    if !server.security_flags.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Security Assessment:",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        for flag in &server.security_flags {
            let badge = flag.badge();
            let desc = flag.description();
            let color = severity_color(&flag.severity());
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", badge),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(desc),
            ]));
        }
    }

    let mut providers: Vec<&str> = server
        .activation
        .iter()
        .filter(|(_, a)| a.global || a.workspace)
        .map(|(pid, _)| pid.as_str())
        .collect();
    providers.sort();
    lines.push(Line::from(vec![
        Span::styled(
            "Active Providers: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(if providers.is_empty() { "none" } else { "—" }),
    ]));
    for p in providers {
        lines.push(Line::from(vec![Span::raw(format!("  • {}", p))]));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}
