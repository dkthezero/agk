use crate::app::snapshot::{DiscoveredMcp, ProviderEntry};
use crate::domain::mcp::McpTransport;
use crate::domain::scope::Scope;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use super::McpState;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &McpState,
    active_selected: usize,
    active_scope: Scope,
    active_providers: &[ProviderEntry],
    discovered_mcps: &[DiscoveredMcp],
) {
    let block = Block::default().borders(Borders::ALL).title("MCP Servers");

    let header = Row::new(vec![
        Cell::from(Span::raw("  ")).style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from(Span::raw("Server")).style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from(Span::raw("Command")).style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from(Span::raw("Transport")).style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from(Span::raw("Tested")).style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let items = state.servers_list();
    let max_cmd_width = area.width as usize / 4;

    let mut rows: Vec<Row> = Vec::new();
    for (i, (id, server)) in items.iter().enumerate() {
        let is_selected = i == active_selected;
        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if server.tested {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let cmd = truncate(&server.command, max_cmd_width);
        let transport = match &server.transport {
            McpTransport::Stdio => "stdio".to_string(),
            McpTransport::Sse { url } => format!("sse: {}", url),
        };
        let tested = if server.tested {
            "[✓]".to_string()
        } else {
            "[ ]".to_string()
        };
        // Checkbox: enabled if any ACTIVE provider has it enabled for this scope
        let enabled = active_providers.iter().any(|p| {
            server
                .activation
                .get(&p.id)
                .map(|a| match active_scope {
                    Scope::Global => a.global,
                    Scope::Workspace => a.workspace,
                })
                .unwrap_or(false)
        });
        let check = if enabled { "[x]" } else { "[ ]" };
        rows.push(
            Row::new(vec![
                Cell::from(Span::raw(check)).style(style),
                Cell::from(Span::raw(id.to_string())).style(style),
                Cell::from(Span::raw(cmd)).style(style),
                Cell::from(Span::raw(transport)).style(style),
                Cell::from(Span::raw(tested)).style(style),
            ])
            .style(style),
        );
    }

    // Append discovered-but-unregistered MCP servers
    if !discovered_mcps.is_empty() {
        let sep_style = Style::default().fg(Color::DarkGray);
        rows.push(Row::new(vec![
            Cell::from(Span::raw("")),
            Cell::from(Span::raw("── Discovered ──")).style(sep_style),
            Cell::from(Span::raw("")),
            Cell::from(Span::raw("")),
            Cell::from(Span::raw("")),
        ]));
        for (i, dm) in discovered_mcps.iter().enumerate() {
            let idx = items.len() + 1 + i; // +1 for separator
            let is_selected = idx == active_selected;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            let desc = dm.description.as_deref().unwrap_or("").trim();
            let label = if desc.is_empty() {
                format!("[⊘] {}", dm.name)
            } else {
                format!("[⊘] {} — {}", dm.name, desc)
            };
            rows.push(
                Row::new(vec![
                    Cell::from(Span::raw("[⊘]").style(style)),
                    Cell::from(Span::raw(label).style(style)),
                    Cell::from(Span::raw(dm.vault_id.clone()).style(style)),
                    Cell::from(Span::raw("").style(style)),
                    Cell::from(Span::raw("").style(style)),
                ])
                .style(style),
            );
        }
    }

    let widths = [
        ratatui::layout::Constraint::Percentage(5),
        ratatui::layout::Constraint::Percentage(20),
        ratatui::layout::Constraint::Percentage(35),
        ratatui::layout::Constraint::Percentage(20),
        ratatui::layout::Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    // Sync highlight to active_selected
    let mut table_state = ratatui::widgets::TableState::default();
    let total_rows = items.len()
        + if !discovered_mcps.is_empty() {
            1 + discovered_mcps.len()
        } else {
            0
        };
    if total_rows > 0 && active_selected < total_rows {
        table_state.select(Some(active_selected));
    }
    frame.render_stateful_widget(table, area, &mut table_state);
}

pub fn render_detail(
    frame: &mut Frame,
    area: Rect,
    state: &McpState,
    active_selected: usize,
    discovered_mcps: &[crate::app::snapshot::DiscoveredMcp],
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

    let mut providers: Vec<&str> = server
        .activation
        .iter()
        .filter(|(_, a)| a.global || a.workspace)
        .map(|(id, _)| id.as_str())
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

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    if end == 0 {
        // max is smaller than the first char's byte length.
        // There is no room for any content before the ellipsis,
        // so just show "..." if we have enough width for it.
        if max >= 3 {
            return String::from("...");
        }
        return String::new();
    }
    format!("{}...", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_ascii_adds_ellipsis() {
        // max=5: keep first 5 bytes then append "..."
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn truncate_max_zero_returns_empty() {
        assert_eq!(truncate("hello", 0), "");
    }

    #[test]
    fn truncate_multibyte_char_boundary() {
        // "héllo" — 'é' is 2 bytes; max=4 cuts at byte 4 (after 'é')
        let s = "héllo world";
        let result = truncate(s, 4);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn truncate_max_inside_first_multibyte_char_with_room_for_ellipsis() {
        // 4-byte emoji; max=3 triggers the end==0 branch with max>=3
        let s = "😀world";
        let result = truncate(s, 3);
        assert_eq!(result, "...");
    }

    #[test]
    fn truncate_max_inside_first_multibyte_char_no_room_for_ellipsis() {
        // 4-byte emoji; max=2 — can't fit any valid output within 2 bytes
        let s = "😀world";
        let result = truncate(s, 2);
        assert_eq!(result, "");
    }
}
