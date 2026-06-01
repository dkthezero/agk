use crate::app::snapshot::{DiscoveredMcp, ProviderEntry};
use crate::domain::mcp::McpTransport;
use crate::domain::mcp_security::{SecurityFlag, SecuritySeverity};
use crate::domain::scope::Scope;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use super::McpState;

/// Return the highest-severity badge string for a set of flags, or empty if none.
fn highest_severity_badge(flags: &[SecurityFlag]) -> &'static str {
    let best = flags.iter().map(|f| f.severity()).max_by_key(|s| match s {
        SecuritySeverity::Low => 0,
        SecuritySeverity::Medium => 1,
        SecuritySeverity::High => 2,
        SecuritySeverity::Critical => 3,
    });
    match best {
        Some(SecuritySeverity::Critical) => "[!!]",
        Some(SecuritySeverity::High) => "[!]",
        Some(SecuritySeverity::Medium) => "[!]",
        Some(SecuritySeverity::Low) => "[i]",
        None => "",
    }
}

fn severity_color(sev: &SecuritySeverity) -> Color {
    match sev {
        SecuritySeverity::Low => Color::Blue,
        SecuritySeverity::Medium => Color::Yellow,
        SecuritySeverity::High => Color::Red,
        SecuritySeverity::Critical => Color::Magenta,
    }
}

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
        Cell::from(Span::raw("Sec")).style(Style::default().add_modifier(Modifier::BOLD)),
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
        let sec_badge = highest_severity_badge(&server.security_flags);
        let sec_style = if server.security_flags.is_empty() {
            style
        } else if let Some(highest) = server
            .security_flags
            .iter()
            .map(SecurityFlag::severity)
            .max_by_key(|s| match s {
                SecuritySeverity::Low => 0,
                SecuritySeverity::Medium => 1,
                SecuritySeverity::High => 2,
                SecuritySeverity::Critical => 3,
            })
        {
            Style::default()
                .fg(severity_color(&highest))
                .add_modifier(Modifier::BOLD)
        } else {
            style
        };
        rows.push(
            Row::new(vec![
                Cell::from(Span::raw(check)).style(style),
                Cell::from(Span::raw(id.to_string())).style(style),
                Cell::from(Span::raw(cmd)).style(style),
                Cell::from(Span::raw(transport)).style(style),
                Cell::from(Span::raw(tested)).style(style),
                Cell::from(Span::raw(sec_badge)).style(sec_style),
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
                dm.name.clone()
            } else {
                format!("{} — {}", dm.name, desc)
            };
            rows.push(
                Row::new(vec![
                    Cell::from(Span::raw("[⊘]").style(style)),
                    Cell::from(Span::raw(label).style(style)),
                    Cell::from(Span::raw(dm.vault_id.clone()).style(style)),
                    Cell::from(Span::raw("").style(style)),
                    Cell::from(Span::raw("").style(style)),
                    Cell::from(Span::raw("").style(style)),
                ])
                .style(style),
            );
        }
    }

    let widths = [
        ratatui::layout::Constraint::Percentage(5),
        ratatui::layout::Constraint::Percentage(18),
        ratatui::layout::Constraint::Percentage(32),
        ratatui::layout::Constraint::Percentage(18),
        ratatui::layout::Constraint::Percentage(12),
        ratatui::layout::Constraint::Percentage(10),
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
