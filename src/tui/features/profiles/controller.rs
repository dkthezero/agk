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
    state.pending_delete_profile = Some(name);
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

/// Enter the export profile modal (Ctrl+E on Profile tab).
pub fn enter_export_profile(state: &mut AppState) {
    if state.profile_entries.is_empty() || state.selected_index >= state.profile_entries.len() {
        state.status_line = "No profile selected to export".to_string();
        return;
    }
    let name = state.profile_entries[state.selected_index].name.clone();
    state.export_file_path = format!("./{}.agk.json", name);
    state.export_resolve_vaults = false;
    state.pending_export_profile = Some(name);
    state.list_mode = ListMode::ExportProfile;
    state.status_line.clear();
}

/// Enter the import profile modal (Ctrl+I on Profile tab).
pub fn enter_import_profile(state: &mut AppState) {
    state.import_file_path.clear();
    state.list_mode = ListMode::ImportProfile;
    state.status_line.clear();
}

/// Handle key input in the export profile modal.
pub fn handle_export_profile_input(
    state: &mut AppState,
    ctx: &EventContext,
    key: &crossterm::event::KeyEvent,
) -> Result<ControlFlow> {
    use crossterm::event::KeyCode;

    match key.code {
        KeyCode::Esc => {
            state.pending_export_profile = None;
            state.export_file_path.clear();
            state.list_mode = ListMode::Normal;
            state.status_line = "Export cancelled".to_string();
            return Ok(ControlFlow::Continue);
        }
        KeyCode::Enter => {
            let name = match state.pending_export_profile.take() {
                Some(n) => n,
                None => {
                    state.list_mode = ListMode::Normal;
                    return Ok(ControlFlow::Continue);
                }
            };
            let file_path = if state.export_file_path.is_empty() {
                format!("./{}.agk.json", name)
            } else {
                state.export_file_path.clone()
            };
            let resolve_vaults = state.export_resolve_vaults;
            let _ = ctx.tx.send(AppEvent::ExecuteCommand(
                crate::app::command::CoreCommand::ExportProfile {
                    profile_id: crate::domain::profile::ProfileId::new(name),
                    scope: state.active_scope,
                    file_path: Some(file_path),
                    resolve_vaults,
                },
            ));
            state.export_file_path.clear();
            state.list_mode = ListMode::Normal;
        }
        KeyCode::Tab => {
            state.export_resolve_vaults = !state.export_resolve_vaults;
        }
        KeyCode::Backspace => {
            state.export_file_path.pop();
        }
        KeyCode::Char(c) => {
            state.export_file_path.push(c);
        }
        _ => {}
    }
    Ok(ControlFlow::Continue)
}

/// Handle key input in the import profile modal.
pub fn handle_import_profile_input(
    state: &mut AppState,
    ctx: &EventContext,
    key: &crossterm::event::KeyEvent,
) -> Result<ControlFlow> {
    use crossterm::event::KeyCode;

    match key.code {
        KeyCode::Esc => {
            state.import_file_path.clear();
            state.list_mode = ListMode::Normal;
            state.status_line = "Import cancelled".to_string();
            return Ok(ControlFlow::Continue);
        }
        KeyCode::Enter => {
            if state.import_file_path.is_empty() {
                state.status_line = "File path cannot be empty".to_string();
                return Ok(ControlFlow::Continue);
            }
            let file_path = state.import_file_path.clone();
            let scope = state.active_scope;
            let _ = ctx.tx.send(AppEvent::ExecuteCommand(
                crate::app::command::CoreCommand::ImportProfile {
                    file_path,
                    target_name: None,
                    scope,
                },
            ));
            state.import_file_path.clear();
            state.list_mode = ListMode::Normal;
        }
        KeyCode::Backspace => {
            state.import_file_path.pop();
        }
        KeyCode::Char(c) => {
            state.import_file_path.push(c);
        }
        _ => {}
    }
    Ok(ControlFlow::Continue)
}
