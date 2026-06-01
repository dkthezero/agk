use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

// Re-export the modal renderers that live in split files (extracted for
// ADR-001 §6.4 file-size compliance) so callers continue to use
// `crate::tui::widgets::modal::render_review_modal` / `render_checklist_modal`.
pub use crate::tui::widgets::modal_checklist::render_checklist_modal;
pub use crate::tui::widgets::modal_long::render_review_modal;

/// Estimate how many display lines `text` will occupy when wrapped to `width`.
pub(super) fn estimate_wrapped_lines(text: &str, width: u16) -> u16 {
    let w = width.max(1);
    text.lines()
        .map(|line| {
            let len = line.chars().count() as u16;
            len.div_ceil(w)
        })
        .sum::<u16>()
        .max(1)
}

/// Render a centered selection modal with a title and list of options.
pub fn render_select_modal(
    frame: &mut Frame,
    title: &str,
    options: &[(String, String)],
    selected: usize,
) {
    let area = frame.area();
    let width = (area.width as f32 * 0.6).clamp(30.0, 60.0) as u16;
    let height = (options.len() as u16 + 4).min(area.height.saturating_sub(4));
    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, (folder, desc))| {
            let style = if i == selected {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let text = format!("{} — {}", folder, desc);
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

/// Render a centered text-input modal with a title, field label, and current value.
pub fn render_input_modal(
    frame: &mut Frame,
    title: &str,
    label: &str,
    value: &str,
    cursor_pos: usize,
) {
    let area = frame.area();
    // Use up to 80% width so long answers don't overflow.
    let width = (area.width as f32 * 0.8).clamp(40.0, 100.0) as u16;
    let inner_width = width.saturating_sub(4).max(1);

    // Compute height from content: label (1) + blank (1) + wrapped value lines + padding (2).
    let value_lines = estimate_wrapped_lines(value, inner_width);
    let height = (value_lines + 4).min(area.height.saturating_sub(4)).max(6);
    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        label,
        Style::default().fg(Color::White),
    )]));
    lines.push(Line::from(""));

    // Split value by newlines so we can place the cursor on the correct line.
    // `cursor_pos` is in **character indices**.
    let value_text = value; // alias for clarity
    let mut char_count = 0usize;
    let mut cursor_line = 0usize;
    let mut cursor_col = 0usize;
    let mut rendered_cursor = false;

    for (line_no, line_text) in value_text.split('\n').enumerate() {
        if !rendered_cursor {
            let line_char_len = line_text.chars().count();
            if char_count + line_char_len >= cursor_pos {
                cursor_line = line_no;
                cursor_col = cursor_pos.saturating_sub(char_count);
                rendered_cursor = true;
            }
            char_count += line_char_len;
            // Account for the newline character itself
            if char_count == cursor_pos {
                cursor_line = line_no + 1;
                cursor_col = 0;
                rendered_cursor = true;
            }
            char_count += 1; // the '\n' char
        }

        let mut spans: Vec<Span> = Vec::new();
        let mut col_idx = 0usize;
        for ch in line_text.chars() {
            if line_no == cursor_line && col_idx == cursor_col {
                spans.push(Span::styled(
                    "█",
                    Style::default().bg(Color::Cyan).fg(Color::Black),
                ));
            }
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(Color::Cyan),
            ));
            col_idx += 1;
        }
        if line_no == cursor_line && col_idx == cursor_col {
            spans.push(Span::styled(
                "█",
                Style::default().bg(Color::Cyan).fg(Color::Black),
            ));
        }
        lines.push(Line::from(spans));
    }

    // If cursor is beyond all content (at the very end of the last line of the last char)
    if !rendered_cursor {
        lines.push(Line::from(vec![Span::styled(
            "█",
            Style::default().bg(Color::Cyan).fg(Color::Black),
        )]));
    }

    let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Parse a keybinds string like "[Enter] Confirm  [Esc] Cancel" into colored spans.
/// Keys (inside `[]`) are shown in Cyan; everything else is DarkGray.
pub(super) fn color_keys(input: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut in_bracket = false;

    for ch in input.chars() {
        if ch == '[' && !in_bracket {
            if !current.is_empty() {
                spans.push(Span::styled(
                    current.clone(),
                    Style::default().fg(Color::DarkGray),
                ));
                current.clear();
            }
            in_bracket = true;
            current.push(ch);
        } else if ch == ']' && in_bracket {
            current.push(ch);
            spans.push(Span::styled(
                current.clone(),
                Style::default().fg(Color::Cyan),
            ));
            current.clear();
            in_bracket = false;
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        let color = if in_bracket {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        spans.push(Span::styled(current, Style::default().fg(color)));
    }
    spans
}

/// Render a centered confirmation modal with a title, message, and action buttons.
///
/// Actions are split by `  ` (double-space) into individual buttons, then laid
/// out horizontally and centered across the full modal width.
pub fn render_confirm_modal(frame: &mut Frame, title: &str, message: &str, actions: &str) {
    let area = frame.area();
    let width = (area.width as f32 * 0.6).clamp(30.0, 70.0) as u16;
    let inner_width = width.saturating_sub(4); // borders + margin
    let msg_lines = estimate_wrapped_lines(message, inner_width.max(1));
    let action_lines = estimate_wrapped_lines(actions, inner_width.max(1));
    let height = (msg_lines + action_lines + 4).min(area.height.saturating_sub(4));
    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    // Compute inner area before rendering the block so we can use it for layout
    let inner = block.inner(popup).inner(Margin::new(1, 1));

    frame.render_widget(block, popup);

    // Split vertically: message at top (exact height), flexible spacer, actions at bottom (exact height)
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(msg_lines),
            Constraint::Min(0),
            Constraint::Length(action_lines),
        ])
        .split(inner);

    let msg_paragraph = Paragraph::new(message)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    frame.render_widget(msg_paragraph, vertical[0]);

    // ── Horizontally distribute action buttons ──────────────────────────
    let button_texts: Vec<&str> = actions.split("  ").map(str::trim).collect();
    let constraints: Vec<Constraint> = std::iter::repeat_n(
        Constraint::Ratio(1, button_texts.len() as u32),
        button_texts.len(),
    )
    .collect();
    let horizontal = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints(constraints)
        .split(vertical[2]);

    for (i, text) in button_texts.iter().enumerate() {
        let spans = color_keys(text);
        let btn = Paragraph::new(Line::from(spans))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        frame.render_widget(btn, horizontal[i]);
    }
}

pub(super) fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Length((r.height.saturating_sub(height)) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Length((r.width.saturating_sub(width)) / 2),
        ])
        .split(popup_layout[1])[1]
}
