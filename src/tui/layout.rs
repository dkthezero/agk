use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub header: Rect,
    pub tabs: Rect,
    pub list: Rect,
    pub detail: Rect,
    pub footer: Rect,
}

pub fn compute(area: Rect) -> AppLayout {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Length(1), // tab bar
            Constraint::Min(1),    // list + detail
            Constraint::Length(2), // footer
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(vertical[2]);

    AppLayout {
        header: vertical[0],
        tabs: vertical[1],
        list: horizontal[0],
        detail: horizontal[1],
        footer: vertical[3],
    }
}
