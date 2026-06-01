use crate::tui::app::AppState;
use crate::tui::event::{AppEvent, ControlFlow, EventContext};
use crate::tui::list_mode::ListMode;
use anyhow::Result;

pub use crate::tui::features::profiles::wizard::handle_profile_wizard_input;

// For use from common/controller handle_backspace
pub fn handle_delete_profile_no_ctx(state: &mut AppState) -> Result<ControlFlow> {
    let filtered_len = state.profile_entries.len();
    if filtered_len == 0 || state.selected_index >= filtered_len {
        state.status_line = "No profile selected to delete".to_string();
        return Ok(ControlFlow::Continue);
    }
    let name = state.profile_entries[state.selected_index].name.clone();
    state.pending_delete_profile = Some(name.clone());
    state.list_mode = ListMode::ConfirmDeleteProfile;
    Ok(ControlFlow::Continue)
}

pub fn handle_delete_profile(state: &mut AppState, _ctx: &EventContext) -> Result<ControlFlow> {
    handle_delete_profile_no_ctx(state)
}

pub fn handle_delete_profile_confirm(
    state: &mut AppState,
    ctx: &EventContext,
) -> Result<ControlFlow> {
    let name = match state.pending_delete_profile.take() {
        Some(n) => n,
        None => {
            state.list_mode = ListMode::Normal;
            return Ok(ControlFlow::Continue);
        }
    };

    let _ = ctx.tx.send(AppEvent::ExecuteCommand(
        crate::app::command::CoreCommand::DeleteProfile {
            id: crate::domain::profile::ProfileId::new(name),
            scope: state.active_scope,
        },
    ));
    state.list_mode = ListMode::Normal;
    Ok(ControlFlow::Continue)
}
