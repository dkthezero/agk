use crate::tui::app::AppState;
use crate::tui::event::{AppEvent, ControlFlow, EventContext};
use crate::tui::list_mode::ListMode;
use anyhow::Result;
use crossterm::event::KeyCode;

pub fn handle_navigation(state: &mut AppState, code: &KeyCode) {
    match code {
        KeyCode::Up if state.selected_index > 0 => {
            state.selected_index -= 1;
            state.scroll_offset = 0;
            state.scroll_tick = 0;
        }
        KeyCode::Down if state.selected_index + 1 < state.list_length() => {
            state.selected_index += 1;
            state.scroll_offset = 0;
            state.scroll_tick = 0;
        }
        _ => {}
    }
}

pub fn handle_esc(state: &mut AppState) -> Result<ControlFlow> {
    if state.is_attach_vault_mode() {
        state.list_mode = ListMode::Normal;
        state.prompt_buffer.clear();
        state.status_line = "Cancelled".to_string();
        return Ok(ControlFlow::Continue);
    }

    if state.is_register_mcp_mode() || state.list_mode == ListMode::ConfirmMcpTest {
        state.list_mode = ListMode::Normal;
        state.prompt_buffer.clear();
        state.status_line = "Cancelled MCP registration".to_string();
        return Ok(ControlFlow::Continue);
    }

    let active_kind = state.tab_kinds.get(state.active_tab).copied();
    if state.list_mode == ListMode::Normal && state.search_query.is_empty() {
        if state.esc_pressed_once {
            return Ok(ControlFlow::Quit);
        }
        state.esc_pressed_once = true;
        state.status_line = "Press ESC again to quit".to_string();
    } else if active_kind != Some(crate::app::tab_kind::TabKind::Vault) {
        apply_esc(state);
    }
    Ok(ControlFlow::Continue)
}

pub fn handle_backspace(state: &mut AppState) {
    let active_kind = state.tab_kinds.get(state.active_tab).copied();
    if state.is_attach_vault_mode() || state.is_register_mcp_mode() {
        state.prompt_buffer.pop();
    } else if active_kind == Some(crate::app::tab_kind::TabKind::Profile) {
        // MacBook "Delete" key produces Backspace; treat it as profile deletion
        if matches!(state.list_mode, ListMode::Normal) {
            let _ = crate::tui::features::profiles::controller::handle_delete_profile_no_ctx(state);
        }
    } else if active_kind != Some(crate::app::tab_kind::TabKind::Vault) {
        state.search_query.pop();
        if state.search_query.is_empty() {
            state.list_mode = ListMode::Normal;
            state.remote_packages.clear();
            if let Some(id) = state.clawhub_search_task_id.take() {
                state.active_tasks.remove(&id);
            }
        }
        state.selected_index = 0;
    }
}

pub fn handle_f_keys(state: &mut AppState, ctx: &EventContext, code: &KeyCode) -> Result<()> {
    match code {
        KeyCode::F(5) => crate::tui::features::assets::controller::handle_f5_update_all(state, ctx),
        KeyCode::F(4) => {
            let _ = ctx.tx.send(AppEvent::ExecuteCommand(
                crate::app::command::CoreCommand::RefreshAllVaults,
            ));
            Ok(())
        }
        KeyCode::F(2) => {
            let vaults_idx = state
                .tab_names
                .iter()
                .position(|n| n == "Vaults")
                .unwrap_or(0);
            let mcp_idx = state
                .tab_names
                .iter()
                .position(|n| n == "MCP Servers")
                .unwrap_or(1);
            let profile_idx = state
                .tab_names
                .iter()
                .position(|n| n == "Profiles")
                .unwrap_or(4);
            if state.active_tab == vaults_idx {
                super::actions::apply_enter_attach_vault(state);
            } else if state.active_tab == mcp_idx {
                super::actions::apply_enter_register_mcp(state);
            } else if state.active_tab == profile_idx {
                super::actions::apply_enter_add_profile(state, ctx);
            }
            Ok(())
        }
        KeyCode::F(3) => {
            if state.tab_kinds.get(state.active_tab)
                == Some(&crate::app::tab_kind::TabKind::Profile)
                && state.list_mode == ListMode::Normal
                && !state.profile_entries.is_empty()
            {
                crate::tui::features::profiles::edit::enter_edit_profile(state, ctx);
            } else if state.tab_kinds.get(state.active_tab)
                == Some(&crate::app::tab_kind::TabKind::Asset)
                && state.list_mode == ListMode::Normal
            {
                crate::tui::features::assets::controller::handle_f3_toggle_team(state, ctx)?;
            }
            Ok(())
        }
        KeyCode::F(1) => {
            let vaults_idx = state
                .tab_names
                .iter()
                .position(|n| n == "Vaults")
                .unwrap_or(0);
            if state.active_tab == vaults_idx && !state.is_vault_workspace {
                crate::tui::features::vaults::controller::enter_vault_init(state, ctx);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn handle_space(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let active_kind = state.tab_kinds.get(state.active_tab).cloned();
    match active_kind {
        Some(crate::app::tab_kind::TabKind::Provider) => {
            crate::tui::features::providers::controller::handle_space_provider(state, ctx)
        }
        Some(crate::app::tab_kind::TabKind::Vault) => {
            crate::tui::features::vaults::controller::handle_space_vault(state, ctx)
        }
        Some(crate::app::tab_kind::TabKind::Mcp) => {
            crate::tui::features::mcps::controller::handle_space_mcp(state, ctx)
        }
        Some(crate::app::tab_kind::TabKind::Asset) => {
            if !state.is_vault_workspace && !state.active_scope_has_provider() {
                let providers_idx = state
                    .tab_names
                    .iter()
                    .position(|n| n == "Providers")
                    .unwrap_or(3);
                super::actions::apply_space_no_provider(state, providers_idx);
                Ok(())
            } else {
                crate::tui::features::assets::controller::handle_space_asset(state, ctx)
            }
        }
        _ => Ok(()),
    }
}

pub fn handle_enter(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let active_kind = state
        .tab_kinds
        .get(state.active_tab)
        .cloned()
        .unwrap_or(crate::app::tab_kind::TabKind::Asset);
    match active_kind {
        crate::app::tab_kind::TabKind::Mcp => {
            // Check if selection is on a discovered MCP
            let registered_count = state.mcp_state.servers_list().len();
            if !state.discovered_mcps.is_empty() && state.selected_index > registered_count {
                crate::tui::features::mcps::controller::handle_enter_discovered_mcp(state)?;
            } else {
                state.status_line = "Press F2 to register a new MCP server".to_string();
            }
        }
        crate::app::tab_kind::TabKind::Asset => {
            if !state.active_scope_has_provider() {
                let providers_idx = state
                    .tab_names
                    .iter()
                    .position(|n| n == "Providers")
                    .unwrap_or(3);
                super::actions::apply_space_no_provider(state, providers_idx);
            } else {
                crate::tui::features::assets::controller::handle_enter_update(state, ctx)?;
            }
        }
        _ => {
            state.status_line = "Update only applies to Skills/Instructions tabs".to_string();
        }
    }
    Ok(())
}

pub fn apply_esc(state: &mut AppState) {
    state.search_query.clear();
    state.list_mode = ListMode::Normal;
    state.selected_index = 0;
    state.remote_packages.clear();
    if let Some(id) = state.clawhub_search_task_id.take() {
        state.active_tasks.remove(&id);
    }
}

pub fn handle_open_location(
    state: &mut AppState,
    ctx: &EventContext,
    in_terminal: bool,
) -> Result<()> {
    let active_kind = state
        .tab_kinds
        .get(state.active_tab)
        .copied()
        .unwrap_or(crate::app::tab_kind::TabKind::Asset);

    if active_kind != crate::app::tab_kind::TabKind::Asset {
        state.status_line = "Ctrl+O/T only works in Skills/Instructions tab".to_string();
        return Ok(());
    }

    let filtered = state.filtered_packages();
    let Some(pkg) = filtered.get(state.selected_index).copied() else {
        state.status_line = "No item selected".to_string();
        return Ok(());
    };

    if pkg.is_remote {
        state.status_line = "Only local packages can be opened".to_string();
        return Ok(());
    }
    if pkg.path.as_os_str().is_empty() {
        state.status_line = "Selected package has no local path".to_string();
        return Ok(());
    }

    let path = &pkg.path;
    let result = if in_terminal {
        ctx.file_opener.open_terminal(path)
    } else {
        ctx.file_opener.open_file_manager(path)
    };

    match result {
        Ok(()) => {
            state.status_line = if in_terminal {
                format!("Opening terminal at {}", path.display())
            } else {
                format!("Opening {} in file manager", path.display())
            };
        }
        Err(e) => {
            state.status_line = format!("Failed to open: {}", e);
        }
    }
    Ok(())
}
