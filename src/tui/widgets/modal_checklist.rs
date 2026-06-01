//! Checklist modal renderer split off `modal.rs` for ADR-001 §6.4 file-size
//! compliance.
//!
//! Re-exported by `modal.rs` so callers can still use
//! `crate::tui::widgets::modal::render_checklist_modal`.

use crate::tui::widgets::modal::{centered_rect, color_keys};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

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
