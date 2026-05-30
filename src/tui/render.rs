mod content;
mod header;
mod keybinds;
mod modals;

use crate::tui::app::AppState;
use crate::tui::layout;
use crate::tui::widgets::{operations, status, tabs};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, state: &AppState) {
    let layout = layout::compute(frame.area(), !state.active_tasks.is_empty());

    header::draw_header(frame, state, layout.header);
    tabs::render(frame, layout.tabs, &state.tab_names, state.active_tab);
    content::draw_content(frame, state, layout.list, layout.detail);
    if let Some(ops_area) = layout.operations {
        operations::draw::draw(frame, state, ops_area);
    }
    let keybinds = keybinds::resolve_keybinds(state);
    status::render(
        frame,
        layout.footer,
        &state.status_line,
        &state.search_query,
        keybinds,
        state.scope_label(),
        state.progress_summary().as_deref(),
    );
    modals::draw_modals(frame, state);
}
