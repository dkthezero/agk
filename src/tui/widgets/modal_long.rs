//! Large modal renderers split off `modal.rs` for ADR-001 §6.4 file-size
//! compliance.
//!
//! Hosts the review-step modal (multi-section profile summary with scroll
//! support) and the checklist modal (filterable multi-select). Both are
//! re-exported by `modal.rs` so callers can still use
//! `crate::tui::widgets::modal::render_*`.

use crate::tui::widgets::modal::{centered_rect, color_keys, estimate_wrapped_lines};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Frame,
};

/// Render a centered review modal showing profile summary with scrolling.
///
/// Content includes Profile name, Description (wrapped), Skills, MCPs, and
/// keybinding hints. When content overflows the available height, a
/// scrollbar appears and `scroll_offset` controls the vertical position.
#[allow(clippy::too_many_arguments)]
pub fn render_review_modal(
    frame: &mut Frame,
    title: &str,
    name: &str,
    description: &str,
    skills: &[String],
    mcps: &[String],
    tools: &[String],
    permission_mode: Option<&str>,
    actions: &str,
    scroll_offset: usize,
) {
    let area = frame.area();
    let width = (area.width as f32 * 0.8).clamp(40.0, 100.0) as u16;
    let inner_width = width.saturating_sub(4).max(1);

    let desc_lines = estimate_wrapped_lines(description, inner_width);
    let skills_lines = if skills.is_empty() {
        1
    } else {
        estimate_wrapped_lines(&skills.join(", "), inner_width)
    };
    let mcps_lines = if mcps.is_empty() {
        1
    } else {
        estimate_wrapped_lines(&mcps.join(", "), inner_width)
    };
    let tools_lines = if tools.is_empty() {
        1
    } else {
        estimate_wrapped_lines(&tools.join(", "), inner_width)
    };

    // Content height: label + content + spacing for each section
    let content_height = 1 // Profile: name
        + 2                 // blank + Description label
        + desc_lines
        + 2                 // blank + Skills label
        + skills_lines
        + 2                 // blank + MCPs label
        + mcps_lines
        + 2                 // blank + Tools label
        + tools_lines
        + 2                 // blank + Permission label
        + 2; // blank + actions padding

    // Cap outer height at 90% of terminal (leave 5% margin top+bottom)
    let max_height = (area.height * 90) / 100;
    let height = content_height.max(12).min(max_height);
    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup).inner(Margin::new(1, 1));
    frame.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Profile: ", Style::default().fg(Color::White)),
        Span::styled(name, Style::default().fg(Color::Cyan)),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        "Description:",
        Style::default().fg(Color::White),
    )]));
    if description.trim().is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "(none)",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        for line in description.lines() {
            lines.push(Line::from(vec![Span::styled(
                line,
                Style::default().fg(Color::Cyan),
            )]));
        }
    }
    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        "Skills:",
        Style::default().fg(Color::White),
    )]));
    if skills.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "(none)",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            skills.join(", "),
            Style::default().fg(Color::Cyan),
        )]));
    }
    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        "MCPs:",
        Style::default().fg(Color::White),
    )]));
    if mcps.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "(none)",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            mcps.join(", "),
            Style::default().fg(Color::Cyan),
        )]));
    }
    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        "Tools:",
        Style::default().fg(Color::White),
    )]));
    if tools.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "(none)",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            tools.join(", "),
            Style::default().fg(Color::Cyan),
        )]));
    }
    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        "Permission Mode:",
        Style::default().fg(Color::White),
    )]));
    lines.push(Line::from(vec![Span::styled(
        permission_mode.unwrap_or("(default)"),
        Style::default().fg(Color::Cyan),
    )]));
    lines.push(Line::from(""));

    let action_spans = color_keys(actions);
    lines.push(Line::from(action_spans));

    // Clamp scroll offset so we don't scroll past the end
    let total_lines = lines.len() as u16;
    let visible_lines = inner.height;
    let max_scroll = total_lines.saturating_sub(visible_lines) as usize;
    let scroll_offset = scroll_offset.min(max_scroll);

    let paragraph = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset as u16, 0));

    frame.render_widget(paragraph, inner);

    // Render scrollbar only when content overflows
    if total_lines > visible_lines {
        let mut scrollbar_state = ScrollbarState::new(total_lines as usize).position(scroll_offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_symbol("█")
            .track_symbol(Some("│"));
        frame.render_stateful_widget(
            scrollbar,
            popup.inner(Margin::new(0, 1)),
            &mut scrollbar_state,
        );
    }
}

/// Render a centered checklist modal with a title, options, check states, selection, and a filter hint.
pub fn render_checklist_modal(
    frame: &mut Frame,
    title: &str,
    options: &[String],
    checked: &[bool],
    selected: usize,
    filter_query: &str,
) {
    let area = frame.area();
    let width = (area.width as f32 * 0.6).clamp(30.0, 60.0) as u16;
    // Reserve space for title, border, filter line, hint line, and options list.
    let visible_count = (area.height.saturating_sub(8) as usize)
        .max(3)
        .min(options.len());
    let height = ((visible_count as u16) + 5).min(area.height.saturating_sub(4));
    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Vertical layout: filter line, hint line, options list
    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // filter query
            Constraint::Length(1), // hint
            Constraint::Min(0),    // options list
        ])
        .split(inner);

    // Show filter query
    let filter_text = if filter_query.is_empty() {
        "Type to filter...".to_string()
    } else {
        format!("Filter: {}", filter_query)
    };
    let filter_style = if filter_query.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let filter_paragraph = Paragraph::new(filter_text).style(filter_style);
    frame.render_widget(filter_paragraph, inner_layout[0]);

    // Hotkey hint
    let hint_spans = color_keys("[Space] Select  [↑/↓] Navigate  [Enter] Confirm  [Esc] Cancel");
    let hint_paragraph = Paragraph::new(Line::from(hint_spans));
    frame.render_widget(hint_paragraph, inner_layout[1]);

    let list_area = inner_layout[2];

    // Determine which slice of options to show based on selected index and visible_count
    let max_scroll = options.len().saturating_sub(visible_count);
    let scroll = selected.min(max_scroll);
    let start = scroll;
    let end = (start + visible_count).min(options.len());

    let items: Vec<ListItem> = options[start..end]
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let global_idx = start + i;
            let marker = if checked.get(global_idx) == Some(&true) {
                "[x]"
            } else {
                "[ ]"
            };
            let text = format!("{} {}", marker, label);
            let style = if global_idx == selected {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);
}
