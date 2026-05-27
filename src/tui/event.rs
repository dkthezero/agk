use crate::tui::app::{AppState, ListMode};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};

pub enum ControlFlow {
    Continue,
    Quit,
}

/// Snapshot produced by a background `reload_state` and sent atomically to the
/// async event loop via `AppEvent::ReloadComplete`.
#[derive(Debug)]
pub struct ReloadSnapshot {
    pub vault_entries: Vec<crate::app::snapshot::VaultEntry>,
    pub provider_entries: Vec<crate::app::snapshot::ProviderEntry>,
    pub profile_entries: Vec<crate::app::snapshot::ProfileEntry>,
    pub packages: std::collections::HashMap<usize, Vec<crate::domain::asset::ScannedPackage>>,
    pub configs:
        std::collections::HashMap<crate::domain::scope::Scope, crate::domain::config::ConfigFile>,
    pub mcp_state: crate::tui::widgets::mcp::McpState,
}

pub enum AppEvent {
    /// Keyboard events from `crossterm` — matched in runtime_loop.rs but never
    /// constructed because the TUI uses direct crossterm polling instead of the
    /// async channel for keyboard input.
    Input(crossterm::event::Event),
    TaskStarted {
        id: usize,
        name: String,
    },
    TaskProgress {
        id: usize,
        percent: u8,
    },
    TaskCompleted {
        id: usize,
        message: String,
    },
    TaskFailed {
        id: usize,
        error: String,
    },
    TriggerReload,
    VaultRefreshRequired {
        id: String,
        config: crate::domain::config::VaultConfig,
    },
    ClawHubSearchResults {
        packages: Vec<crate::domain::asset::ScannedPackage>,
        task_id: usize,
    },
    Tick,
    /// Background reload finished atomically so the UI never freezes.
    ReloadComplete(ReloadSnapshot),
    /// then resume TUI. The child runs interactively (user can type/respond).
    RunInteractiveProcess {
        command: String,
        args: Vec<String>,
        current_dir: std::path::PathBuf,
        profile_name: Option<String>,
    },
}

pub struct EventContext {
    pub store: Arc<dyn crate::app::ports::ConfigStorePort>,
    pub registry: Arc<crate::app::registry::Registry>,
    pub tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    pub workspace_root: std::path::PathBuf,
}

use std::sync::Arc;

fn is_clawhub_active(ctx: &EventContext) -> bool {
    ctx.store
        .load(crate::domain::scope::Scope::Global)
        .map(|c| c.vaults.contains(&"clawhub".to_string()))
        .unwrap_or(false)
}

pub fn handle(
    state: &mut AppState,
    ctx: &EventContext,
    evt: crossterm::event::Event,
) -> Result<ControlFlow> {
    if let crossterm::event::Event::Key(key) = evt {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(ControlFlow::Quit);
        }
        if key.code != KeyCode::Esc {
            state.esc_pressed_once = false;
        }

        // Short-circuit when in SelectProviderRoot modal
        if matches!(state.list_mode, ListMode::SelectProviderRoot { .. }) {
            return crate::tui::features::providers::controller::handle_select_provider_root(
                state, ctx, &key.code,
            );
        }

        // Wizard/input modes
        if state.is_attach_vault_mode() {
            return crate::tui::features::vaults::controller::handle_attach_vault_input(
                state, ctx, &key.code,
            )
            .map(|_| ControlFlow::Continue);
        }
        if state.is_register_mcp_mode() {
            return crate::tui::features::mcps::controller::handle_register_mcp_input(
                state, ctx, &key.code,
            )
            .map(|_| ControlFlow::Continue);
        }
        if state.is_profile_wizard_mode()
            && matches!(
                key.code,
                KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Char(' ')
                    | KeyCode::Char(_)
                    | KeyCode::Backspace
                    | KeyCode::Enter
            )
        {
            return crate::tui::features::profiles::controller::handle_profile_wizard_input(
                state, ctx, &key,
            )
            .map(|_| ControlFlow::Continue);
        }

        // Confirmation modals share Enter / Esc aliases
        let in_confirm = matches!(
            state.list_mode,
            ListMode::ConfirmMcpTest
                | ListMode::ConfirmClawHubInstall
                | ListMode::ConfirmDetachVault
                | ListMode::ConfirmDeactivateLastProvider
                | ListMode::ConfirmDeleteProfile
        );

        if key.code == KeyCode::Enter && in_confirm {
            return match state.list_mode {
                ListMode::ConfirmMcpTest => {
                    crate::tui::features::mcps::controller::handle_mcp_register_confirm(state, ctx)
                }
                ListMode::ConfirmClawHubInstall => {
                    crate::tui::features::vaults::controller::handle_clawhub_install_confirm(state, ctx)
                }
                ListMode::ConfirmDetachVault => {
                    crate::tui::features::vaults::controller::handle_detach_confirm(state, ctx)
                }
                ListMode::ConfirmDeactivateLastProvider => {
                    crate::tui::features::providers::controller::handle_deactivate_last_provider_confirm(state, ctx)
                }
                ListMode::ConfirmDeleteProfile => {
                    crate::tui::features::profiles::controller::handle_delete_profile_confirm(state, ctx)
                }
                _ => Ok(ControlFlow::Continue),
            };
        }

        if key.code == KeyCode::Esc && in_confirm {
            return match state.list_mode {
                ListMode::ConfirmMcpTest => {
                    state.list_mode = ListMode::Normal;
                    state.status_line = "Cancelled MCP registration".to_string();
                    Ok(ControlFlow::Continue)
                }
                ListMode::ConfirmClawHubInstall => {
                    state.list_mode = ListMode::Normal;
                    state.status_line = "Cancelled ClawHub CLI install".to_string();
                    Ok(ControlFlow::Continue)
                }
                ListMode::ConfirmDetachVault => {
                    crate::tui::features::vaults::controller::handle_detach_cancel(state)
                }
                ListMode::ConfirmDeactivateLastProvider => {
                    crate::tui::features::providers::controller::handle_deactivate_last_provider_cancel(state)
                }
                ListMode::ConfirmDeleteProfile => {
                    state.list_mode = ListMode::Normal;
                    state.pending_delete_profile = None;
                    state.status_line = "Cancelled profile deletion".to_string();
                    Ok(ControlFlow::Continue)
                }
                _ => Ok(ControlFlow::Continue),
            };
        }

        match key.code {
            KeyCode::Char('0') if state.list_mode == ListMode::Normal => {
                let vault_idx = state.tab_names.len().saturating_sub(1);
                crate::tui::features::common::actions::apply_tab_switch(
                    state,
                    vault_idx,
                    state.tab_names.len(),
                );
            }
            KeyCode::Char(c @ '1'..='5') if state.list_mode == ListMode::Normal => {
                let idx = (c as usize) - ('1' as usize);
                crate::tui::features::common::actions::apply_tab_switch(
                    state,
                    idx,
                    state.tab_names.len(),
                );
            }
            KeyCode::Up | KeyCode::Down => {
                crate::tui::features::common::controller::handle_navigation(state, &key.code);
            }
            KeyCode::Esc => {
                return crate::tui::features::common::controller::handle_esc(state);
            }
            KeyCode::Backspace => {
                crate::tui::features::common::controller::handle_backspace(state);
            }
            KeyCode::Char(' ')
                if state.list_mode == ListMode::Normal
                    || state.list_mode == ListMode::Searching =>
            {
                crate::tui::features::common::controller::handle_space(state, ctx)?;
            }
            KeyCode::Enter if state.list_mode == ListMode::Normal => {
                crate::tui::features::common::controller::handle_enter(state, ctx)?;
            }
            KeyCode::F(5) | KeyCode::F(4) | KeyCode::F(2)
                if state.list_mode == ListMode::Normal =>
            {
                crate::tui::features::common::controller::handle_f_keys(state, ctx, &key.code)?;
            }
            KeyCode::Tab if state.list_mode == ListMode::Normal => {
                crate::tui::features::common::actions::apply_scope_toggle(state);
                let _ = ctx.tx.send(AppEvent::TriggerReload);
            }
            KeyCode::Delete if state.list_mode == ListMode::Normal => {
                let active_kind = state.tab_kinds.get(state.active_tab).copied();
                if active_kind == Some(crate::app::tab_kind::TabKind::Profile) {
                    crate::tui::features::profiles::controller::handle_delete_profile(state, ctx)?;
                }
            }
            KeyCode::Char('o')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && state.list_mode == ListMode::Normal =>
            {
                crate::tui::features::common::controller::handle_open_location(state, ctx, false)?;
            }
            KeyCode::Char('t')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && state.list_mode == ListMode::Normal =>
            {
                crate::tui::features::common::controller::handle_open_location(state, ctx, true)?;
            }
            KeyCode::Char(c) => {
                let active_kind = state.tab_kinds.get(state.active_tab).copied();
                if active_kind != Some(crate::app::tab_kind::TabKind::Vault) {
                    crate::tui::features::common::actions::apply_search_char(state, c);
                    if active_kind == Some(crate::app::tab_kind::TabKind::Asset)
                        && is_clawhub_active(ctx)
                        && !state.search_query.is_empty()
                    {
                        crate::tui::features::vaults::controller::dispatch_clawhub_search(
                            state, ctx,
                        );
                    }
                }
            }
            _ => {}
        }
    }
    Ok(ControlFlow::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    fn empty_state(tab_count: usize) -> AppState {
        AppState::new(
            (0..tab_count).map(|i| format!("Tab{}", i)).collect(),
            vec![true; tab_count],
            HashMap::new(),
        )
    }

    #[test]
    fn test_handle_navigation_down() {
        let mut state = empty_state(1);
        state.packages.insert(
            0,
            vec![
                crate::domain::asset::ScannedPackage {
                    identity: crate::domain::identity::AssetIdentity::new("a", None, "hash"),
                    path: std::path::PathBuf::from("a"),
                    vault_id: "v".into(),
                    kind: crate::domain::asset::AssetKind::Skill,
                    is_remote: false,
                    remote_meta: None,
                    requires: vec![],
                    requires_optional: vec![],
                    author: None,
                    description: None,
                    include_evals: false,
                },
                crate::domain::asset::ScannedPackage {
                    identity: crate::domain::identity::AssetIdentity::new("b", None, "hash"),
                    path: std::path::PathBuf::from("b"),
                    vault_id: "v".into(),
                    kind: crate::domain::asset::AssetKind::Skill,
                    is_remote: false,
                    remote_meta: None,
                    requires: vec![],
                    requires_optional: vec![],
                    author: None,
                    description: None,
                    include_evals: false,
                },
            ],
        );
        state.tab_kinds = vec![crate::app::tab_kind::TabKind::Asset];

        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let registry = Arc::new(crate::app::registry::Registry::new());
        let store = Arc::new(crate::infra::config::toml_store::TomlConfigStore::new(
            std::path::PathBuf::from("g"),
            std::path::PathBuf::from("w"),
        ));
        let ctx = EventContext {
            store,
            registry,
            tx,
            workspace_root: std::path::PathBuf::from("."),
        };

        let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::empty(),
        ));
        handle(&mut state, &ctx, event).unwrap();
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_handle_esc_quit() {
        let mut state = empty_state(1);
        state.esc_pressed_once = true;
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let registry = Arc::new(crate::app::registry::Registry::new());
        let store = Arc::new(crate::infra::config::toml_store::TomlConfigStore::new(
            std::path::PathBuf::from("g"),
            std::path::PathBuf::from("w"),
        ));
        let ctx = EventContext {
            store,
            registry,
            tx,
            workspace_root: std::path::PathBuf::from("."),
        };

        let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::empty(),
        ));
        let res = handle(&mut state, &ctx, event).unwrap();
        assert!(matches!(res, ControlFlow::Quit));
    }
}
