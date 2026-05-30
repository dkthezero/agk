use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub header: Rect,
    pub tabs: Rect,
    pub list: Rect,
    pub detail: Rect,
    pub footer: Rect,
    pub operations: Option<Rect>,
}

pub fn compute(area: Rect, has_active_tasks: bool) -> AppLayout {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Length(1), // tab bar
            Constraint::Min(1),    // list + detail + optional ops
            Constraint::Length(2), // footer
        ])
        .split(area);

    let horizontal = if has_active_tasks {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(30),
                Constraint::Percentage(20),
            ])
            .split(vertical[2])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(vertical[2])
    };

    AppLayout {
        header: vertical[0],
        tabs: vertical[1],
        list: horizontal[0],
        detail: horizontal[1],
        footer: vertical[3],
        operations: if has_active_tasks {
            Some(horizontal[2])
        } else {
            None
        },
    }
}
