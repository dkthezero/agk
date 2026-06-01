use crate::tui::app::AppState;
use crate::tui::widgets::modal::{centered_rect, color_keys, estimate_wrapped_lines};
use ratatui::{
    layout::Margin,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Render a centered modal for exporting a profile.
///
/// Shows the profile name being exported, a text input for the output file
/// path, and a toggle for `--resolve-vaults`.
pub fn render_export_profile_modal(frame: &mut Frame, state: &AppState) {
    let profile_name = state
        .pending_export_profile
        .as_deref()
        .unwrap_or("(unknown)");
    let area = frame.area();
    let width = (area.width as f32 * 0.7).clamp(40.0, 80.0) as u16;
    let inner_width = width.saturating_sub(4).max(1);

    let path_lines = estimate_wrapped_lines(&state.export_file_path, inner_width).max(1) as usize;
    let content_height = 2 // title + blank
        + 1 // "Profile:" label
        + 1 // blank
        + 1 // "File path:" label
        + path_lines
        + 1 // blank
        + 1 // resolve-vaults toggle line
        + 2; // blank + key hints
    let max_height = ((area.height as usize) * 90) / 100;
    let height = content_height.max(10).min(max_height) as u16;
    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title("Export Profile")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup).inner(Margin::new(1, 1));
    frame.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();

    // Profile name
    lines.push(Line::from(vec![
        Span::styled("Profile: ", Style::default().fg(Color::White)),
        Span::styled(
            profile_name,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // File path label
    lines.push(Line::from(Span::styled(
        "File path:",
        Style::default().fg(Color::White),
    )));

    // File path value with cursor
    let cursor_pos = state.export_file_path.chars().count();
    let mut path_spans: Vec<Span> = Vec::new();
    let mut col = 0usize;
    for ch in state.export_file_path.chars() {
        if col == cursor_pos {
            path_spans.push(Span::styled(
                "█",
                Style::default().bg(Color::Cyan).fg(Color::Black),
            ));
        }
        path_spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(Color::Cyan),
        ));
        col += 1;
    }
    if col == cursor_pos {
        path_spans.push(Span::styled(
            "█",
            Style::default().bg(Color::Cyan).fg(Color::Black),
        ));
    }
    lines.push(Line::from(path_spans));
    lines.push(Line::from(""));

    // Resolve vaults toggle
    let vault_marker = if state.export_resolve_vaults {
        "[Y]"
    } else {
        "[N]"
    };
    let vault_style = if state.export_resolve_vaults {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(vec![
        Span::styled("Resolve vaults: ", Style::default().fg(Color::White)),
        Span::styled(
            format!("{} resolve-vaults", vault_marker),
            vault_style,
        ),
    ]));
    lines.push(Line::from(""));

    // Key hints
    let hint_spans =
        color_keys("[Tab] Toggle resolve-vaults  [Enter] Export  [Esc] Cancel");
    lines.push(Line::from(hint_spans));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}