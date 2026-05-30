use crate::tui::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

/// Draw the operations sidebar showing active background tasks.
pub fn draw(frame: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .title("Operations")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let mut tasks: Vec<&crate::tui::progress::Progress> = state.active_tasks.values().collect();
    tasks.sort_by(|a, b| a.name.cmp(&b.name));

    let items: Vec<ListItem> = tasks
        .into_iter()
        .map(|task| {
            let status_text = match &task.status {
                crate::tui::progress::ProgressStatus::Starting => "⏳ starting".to_string(),
                crate::tui::progress::ProgressStatus::Running(pct) => format!("▶ {:>3}%", pct),
            };
            let line = Line::from(vec![
                ratatui::text::Span::raw(&task.name),
                ratatui::text::Span::raw(" "),
                ratatui::text::Span::styled(status_text, Style::default().fg(Color::Cyan)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
