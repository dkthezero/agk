mod content;
mod header;
mod keybinds;
mod modals;
mod profile_wizard;

use crate::tui::app::AppState;
use crate::tui::layout;
use crate::tui::widgets::{status, tabs};
use crate::tui::widgets::team_badge::team_status_line;
use ratatui::Frame;

pub fn draw(frame: &mut Frame, state: &AppState) {
    let layout = layout::compute(frame.area());

    header::draw_header(frame, state, layout.header);
    tabs::render(frame, layout.tabs, &state.tab_names, state.active_tab);
    content::draw_content(frame, state, layout.list, layout.detail);
    let keybinds = keybinds::resolve_keybinds(state);
    let team_status_str = state.team_status().map(|(installed, required, personal)| {
        team_status_line(installed, required, personal)
    });
    status::render(
        frame,
        layout.footer,
        &state.status_line,
        &state.search_query,
        keybinds,
        state.scope_label(),
        state.progress_summary().as_deref(),
        team_status_str.as_deref(),
    );
    modals::draw_modals(frame, state);
}
