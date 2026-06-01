use crate::app::features::profile::token_estimate::token_badge_color;
use crate::tui::app::EditProfileState;
use crate::tui::widgets::modal::{centered_rect, color_keys};
use ratatui::{
    layout::Margin,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Render a centered modal for editing a profile's skills, MCPs, and
/// permission_mode.
pub fn render_edit_profile_modal(frame: &mut Frame, state: &EditProfileState) {
    let area = frame.area();
    let width = (area.width as f32 * 0.7).clamp(40.0, 90.0) as u16;

    // Estimate content height.
    let skills_lines = state.skills.len().max(1);
    let mcps_lines = state.mcps.len().max(1);
    let perm_lines = state.permission_modes.len().max(1);
    let content_height = 2 // title + blank
        + 1 // "Skills:" label
        + skills_lines
        + 1 // blank
        + 1 // "MCPs:" label
        + mcps_lines
        + 1 // blank
        + 1 // "Permission:" label
        + perm_lines
        + 2; // blank + key hints

    let max_height = ((area.height as usize) * 90) / 100;
    let height = content_height.max(14).min(max_height) as u16;
    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);

    let token_color = match token_badge_color(state.estimated_tokens) {
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "red" => Color::Red,
        _ => Color::White,
    };
    let title_text = format!(
        "Edit Profile: {}  [Est. ~{} tok]",
        state.profile_name, state.estimated_tokens
    );
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(title_text, Style::default().fg(Color::Yellow)),
            Span::styled(" ●", Style::default().fg(token_color)),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup).inner(Margin::new(1, 1));
    frame.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();

    // Skills section
    let skills_header = field_label("Skills", state.field_index == 0);
    lines.push(Line::from(skills_header));

    if state.skills.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no skills available)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, name) in state.skills.iter().enumerate() {
            let checked = state.skills_checked.get(i).copied().unwrap_or(false);
            let marker = if checked { "[x]" } else { "[ ]" };
            let is_active = state.field_index == 0 && state.selected == i;
            let style = if is_active {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(
                format!(" {} {} {}", marker, name, if is_active { " <" } else { "" }),
                style,
            )));
        }
    }
    lines.push(Line::from(""));

    // MCPs section
    let mcps_header = field_label("MCPs", state.field_index == 1);
    lines.push(Line::from(mcps_header));

    if state.mcps.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no MCPs available)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, name) in state.mcps.iter().enumerate() {
            let checked = state.mcps_checked.get(i).copied().unwrap_or(false);
            let marker = if checked { "[x]" } else { "[ ]" };
            let is_active = state.field_index == 1 && state.selected == i;
            let style = if is_active {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(
                format!(" {} {} {}", marker, name, if is_active { " <" } else { "" }),
                style,
            )));
        }
    }
    lines.push(Line::from(""));

    // Permission mode section
    let perm_header = field_label("Permission Mode", state.field_index == 2);
    lines.push(Line::from(perm_header));

    for (i, mode) in state.permission_modes.iter().enumerate() {
        let is_selected = state.permission_index == i;
        let is_active = state.field_index == 2 && state.selected == i;
        let marker = if is_selected { "(*)" } else { "( )" };
        let style = if is_active {
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(if is_selected {
                Color::Cyan
            } else {
                Color::White
            })
        };
        lines.push(Line::from(Span::styled(
            format!(" {} {}{}", marker, mode, if is_active { " <" } else { "" }),
            style,
        )));
    }
    lines.push(Line::from(""));

    // Key hints
    let hint_spans = color_keys("[Tab] Switch field  [Space] Toggle  [Enter] Save  [Esc] Cancel");
    lines.push(Line::from(hint_spans));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Build a field label with optional highlight.
fn field_label(label: &str, active: bool) -> Vec<Span<'_>> {
    if active {
        vec![
            Span::styled(
                format!("▸ {} ", label),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("(active)", Style::default().fg(Color::DarkGray)),
        ]
    } else {
        vec![Span::styled(
            format!("  {} ", label),
            Style::default().fg(Color::White),
        )]
    }
}
