//! Review modal renderer split off `modal.rs` for ADR-001 §6.4 file-size
//! compliance.
//!
//! Hosts the review-step modal (multi-section profile summary with scroll
//! support). Re-exported by `modal.rs` so callers can still use
//! `crate::tui::widgets::modal::render_review_modal`.

use crate::tui::widgets::modal::{centered_rect, color_keys, estimate_wrapped_lines};
use ratatui::{
    layout::Margin,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
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

    // Estimate tokens for the badge
    let token_count = crate::app::features::profile::token_estimate::estimate_tokens(description);
    let token_color_name =
        crate::app::features::profile::token_estimate::token_badge_color(token_count);

    // Content height: label + content + spacing for each section
    let content_height = 1 // Profile: name
        + 2                 // blank + Description label
        + desc_lines
        + 2                 // blank + Token badge
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

    // Token estimate badge
    let token_color = match token_color_name {
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        _ => Color::Red,
    };
    lines.push(Line::from(vec![
        Span::styled("Tokens: ", Style::default().fg(Color::White)),
        Span::styled(
            format!("~{}", token_count),
            Style::default().fg(token_color),
        ),
    ]));
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
