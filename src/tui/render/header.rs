use crate::tui::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
    Frame,
};

pub fn draw_header(frame: &mut Frame, _state: &AppState, area: Rect) {
    let header_text = format!("agk v{}", env!("CARGO_PKG_VERSION"));
    frame.render_widget(
        Paragraph::new(Line::from(header_text)).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        area,
    );
}
