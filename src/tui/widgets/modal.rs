use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Frame,
};

/// Estimate how many display lines `text` will occupy when wrapped to `width`.
fn estimate_wrapped_lines(text: &str, width: u16) -> u16 {
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
fn color_keys(input: &str) -> Vec<Span<'_>> {
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

/// Render a scrollable review modal with structured sections.
///
/// The modal always auto-adjusts its outer height to the content but is
/// capped at 90% of the terminal.  When content exceeds modal height, a
/// scrollbar appears and `scroll_offset` controls the vertical position.
#[allow(clippy::too_many_arguments)]
pub fn render_review_modal(
    frame: &mut Frame,
    title: &str,
    name: &str,
    description: &str,
    skills: &[String],
    mcps: &[String],
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

    // Content height: label + content + spacing for each section
    let content_height = 1 // Profile: name
        + 2                 // blank + Description label
        + desc_lines
        + 2                 // blank + Skills label
        + skills_lines
        + 2                 // blank + MCPs label
        + mcps_lines
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

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
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
