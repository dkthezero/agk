use crate::app::command::CoreCommand;
use crate::app::core::AgkCore;
use crate::app::event::CoreEvent;
use crate::tui::app::AppState;
use crate::tui::list_mode::ListMode;
use crate::tui::reload::ReloadSnapshot;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::Arc;

pub enum ControlFlow {
    Continue,
    Quit,
}

#[derive(Debug)]
pub enum AppEvent {
    /// Keyboard events from `crossterm` forwarded by `main.rs` into the async
    /// channel consumed by `runtime_loop::run_loop`.
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
    /// Execute a [`CoreCommand`] through [`AgkCore`] in a blocking task.
    ExecuteCommand(CoreCommand),
    /// A [`CoreEvent`] emitted by [`AgkCore`] back to the TUI.
    CoreEvent(CoreEvent),
}

pub struct EventContext {
    pub tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    pub workspace_root: std::path::PathBuf,
    pub file_opener: Arc<dyn crate::app::ports::FileOpenerPort>,
    /// Reference to the shared [`AgkCore`] façade so controllers can dispatch
    /// commands and the runtime loop can execute them.
    pub core: Arc<AgkCore>,
}

fn is_clawhub_active(ctx: &EventContext) -> bool {
    ctx.core
        .store
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

        // Profile editor modal
        if matches!(state.list_mode, ListMode::EditProfile) {
            return crate::tui::features::profiles::edit::handle_edit_profile_input(
                state, ctx, &key,
            )
            .map(|_| ControlFlow::Continue);
        }

        // Export profile modal
        if matches!(state.list_mode, ListMode::ExportProfile) {
            return crate::tui::features::profiles::controller::handle_export_profile_input(
                state, ctx, &key,
            )
            .map(|_| ControlFlow::Continue);
        }

        // Import profile modal
        if matches!(state.list_mode, ListMode::ImportProfile) {
            return crate::tui::features::profiles::controller::handle_import_profile_input(
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
            KeyCode::F(5) | KeyCode::F(4) | KeyCode::F(3) | KeyCode::F(2)
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
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && state.list_mode == ListMode::Normal =>
            {
                let active_kind = state.tab_kinds.get(state.active_tab).copied();
                if active_kind == Some(crate::app::tab_kind::TabKind::Profile) {
                    crate::tui::features::profiles::controller::enter_export_profile(state);
                }
            }
            KeyCode::Char('i')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && state.list_mode == ListMode::Normal =>
            {
                let active_kind = state.tab_kinds.get(state.active_tab).copied();
                if active_kind == Some(crate::app::tab_kind::TabKind::Profile) {
                    crate::tui::features::profiles::controller::enter_import_profile(state);
                }
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

    struct StubFileOpener;
    impl crate::app::ports::FileOpenerPort for StubFileOpener {
        fn open_file_manager(&self, _: &std::path::Path) -> anyhow::Result<()> {
            Ok(())
        }
        fn open_terminal(&self, _: &std::path::Path) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn stub_opener() -> Arc<dyn crate::app::ports::FileOpenerPort> {
        Arc::new(StubFileOpener)
    }

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
        let ctx = EventContext {
            tx,
            workspace_root: std::path::PathBuf::from("."),
            file_opener: stub_opener(),
            core: Arc::new(crate::app::core::test_core()),
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
        let ctx = EventContext {
            tx,
            workspace_root: std::path::PathBuf::from("."),
            file_opener: stub_opener(),
            core: Arc::new(crate::app::core::test_core()),
        };

        let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::empty(),
        ));
        let res = handle(&mut state, &ctx, event).unwrap();
        assert!(matches!(res, ControlFlow::Quit));
    }
}
