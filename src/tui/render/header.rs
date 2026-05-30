use crate::tui::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
    Frame,
};

pub fn draw_header(frame: &mut Frame, state: &AppState, area: Rect) {
    let search_hint = if state.search_query.is_empty() {
        String::new()
    } else {
        format!("  [ Search: {} ]", state.search_query)
    };
    let header_text = format!("agk v0.2.7{}", search_hint);
    frame.render_widget(
        Paragraph::new(Line::from(header_text)).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        area,
    );
}
