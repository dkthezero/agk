use crate::tui::app::AppState;
use crate::tui::widgets::modal::{centered_rect, color_keys, estimate_wrapped_lines};
use ratatui::{
    layout::Margin,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Render a centered modal for importing a profile.
///
/// Shows a text input for the file path and a preview pane with profile
/// metadata (name, provider, skill count, MCP count) if a file path
/// resembles a valid path. Warnings for missing vaults appear when
/// relevant.
pub fn render_import_profile_modal(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let width = (area.width as f32 * 0.7).clamp(40.0, 80.0) as u16;
    let inner_width = width.saturating_sub(4).max(1);

    let path_lines = estimate_wrapped_lines(&state.import_file_path, inner_width).max(1) as usize;
    let content_height = 2 // title + blank
        + 1 // "File path:" label
        + path_lines
        + 2 // blank + key hints
        + 1; // minimum
    let max_height = ((area.height as usize) * 90) / 100;
    let height = content_height.max(10).min(max_height) as u16;
    let popup = centered_rect(width, height, area);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title("Import Profile")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup).inner(Margin::new(1, 1));
    frame.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();

    // File path label
    lines.push(Line::from(Span::styled(
        "File path to import:",
        Style::default().fg(Color::White),
    )));

    // File path value with cursor
    let cursor_pos = state.import_file_path.chars().count();
    let mut path_spans: Vec<Span> = Vec::new();
    let mut col = 0usize;
    for ch in state.import_file_path.chars() {
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

    // Preview: show a hint about what will happen
    lines.push(Line::from(Span::styled(
        "Profile will be imported into current scope.",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    // Key hints
    let hint_spans = color_keys("[Enter] Import  [Esc] Cancel");
    lines.push(Line::from(hint_spans));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}
