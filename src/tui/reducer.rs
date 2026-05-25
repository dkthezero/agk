use crate::app::tab_kind::TabKind;
use crate::tui::app_state::{ListMode, TuiState, WizardState};
use crate::tui::intent::UiIntent;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Pure reducer: translates a keyboard event into user intents.
///
/// This function is **stateless and side-effect free**; it does not spawn
/// processes, read files, or send messages.  All it does is inspect the
/// current `TuiState` and produce a list of [`UiIntent`]s that the caller
/// (the TUI main loop or CLI test harness) can later map to
/// [`crate::app::command::CoreCommand`]s and execute.
///
/// ## Elm-style testing
///
/// ```rust
/// let mut state = TuiState::new(vec!["Skills"], vec![true]);
/// let intents = reduce_key(&mut state, KeyEvent::from(KeyCode::F(2)));
/// assert_eq!(intents, vec![UiIntent::OpenCreateProfileWizard]);
/// ```
///
pub fn reduce_key(state: &mut TuiState, key: KeyEvent) -> Vec<UiIntent> {
    let mut intents = Vec::new();

    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return vec![UiIntent::RequestQuit];
    }

    // Any non-Esc key resets the double-press guard
    if key.code != KeyCode::Esc {
        state.esc_pressed_once = false;
    }

    // Modal / wizard navigation takes priority
    if state.list_mode != ListMode::Normal && state.list_mode != ListMode::Searching {
        handle_modal(state, &key.code, &mut intents);
        return intents;
    }

    // Normal / searching mode
    match key.code {
        // Numbered tabs (1–5) + Vault tab (0)
        KeyCode::Char('0') => {
            let vault_idx = state.tab_names.len().saturating_sub(1);
            intents.push(UiIntent::SwitchTab(vault_idx));
        }
        KeyCode::Char(c @ '1'..='5') => {
            let idx = (c as usize) - ('1' as usize);
            intents.push(UiIntent::SwitchTab(idx));
        }

        // Navigation
        KeyCode::Up => intents.push(UiIntent::NavigateUp),
        KeyCode::Down => intents.push(UiIntent::NavigateDown),

        // Search
        KeyCode::Char(c) if c != ' ' => {
            if state.list_mode == ListMode::Normal {
                state.search_query.push(c);
                state.list_mode = ListMode::Searching;
                state.selected_index = 0;
            } else if state.list_mode == ListMode::Searching {
                state.search_query.push(c);
                state.selected_index = 0;
            }
            intents.push(UiIntent::AppendSearchChar(c));
        }
        KeyCode::Backspace => {
            if !state.search_query.is_empty() {
                state.search_query.pop();
                if state.search_query.is_empty() {
                    state.list_mode = ListMode::Normal;
                }
                intents.push(UiIntent::ClearSearch);
            }
        }
        KeyCode::Esc => {
            if state.search_query.is_empty() && state.esc_pressed_once {
                intents.push(UiIntent::RequestQuit);
            } else {
                state.search_query.clear();
                state.list_mode = ListMode::Normal;
                state.selected_index = 0;
                state.esc_pressed_once = true;
            }
        }

        // Action keys
        KeyCode::Enter => {
            if let Some(ListMode::SelectProviderRoot { .. }) = Some(&state.list_mode) {
                // Handled by modal branch above
            } else {
                intents.push(derive_enter_intent(state));
            }
        }
        KeyCode::Char(' ') => {
            intents.push(derive_space_intent(state));
        }

        // Function keys
        KeyCode::F(2) => {
            if is_vault_tab_active(state) {
                intents.push(UiIntent::OpenAttachVaultWizard);
            } else if is_mcp_tab_active(state) {
                // F2 on MCP tab: open register wizard (future)
            } else if is_profile_tab_active(state) {
                intents.push(UiIntent::OpenCreateProfileWizard);
            }
        }
        KeyCode::F(4) => {
            intents.push(UiIntent::RequestReload);
        }
        KeyCode::F(5) => {
            // Update all installed assets
            intents.push(UiIntent::RequestReload);
        }

        // Scope toggle
        KeyCode::Tab => {
            state.toggle_scope();
            intents.push(UiIntent::ToggleScope);
            intents.push(UiIntent::RequestReload);
        }

        _ => {}
    }

    intents
}

// ---------------------------------------------------------------------------
// Modal / wizard handlers
// ---------------------------------------------------------------------------

fn handle_modal(state: &mut TuiState, code: &KeyCode, intents: &mut Vec<UiIntent>) {
    use ListMode::*;

    match code {
        KeyCode::Esc => {
            // Cancel modal
            state.list_mode = ListMode::Normal;
            match &state.list_mode {
                // This branch is unreachable because we just reset, but
                // kept for exhaustiveness during refactor.
                _ => {
                    state.wizard = None;
                    intents.push(UiIntent::CloseModal);
                }
            }
        }
        KeyCode::Enter => match &state.list_mode {
            ConfirmDeleteProfile => {
                if let Some(name) = state.pending_delete_profile.take() {
                    intents.push(UiIntent::DeleteProfile(
                        crate::domain::profile::ProfileId::new(name),
                    ));
                }
                state.list_mode = ListMode::Normal;
            }
            ConfirmDetachVault => {
                let id = std::mem::take(&mut state.pending_vault_id);
                intents.push(UiIntent::DetachVault(id));
                state.list_mode = ListMode::Normal;
            }
            ProfileWizard => {
                if let Some(w) = &state.wizard {
                    let input = crate::app::command::CreateProfileInput::new(
                        crate::domain::profile::ProfileId::new(&w.profile_name),
                        crate::domain::profile::ProviderId::new(&w.provider_id),
                        state.active_scope,
                    );
                    intents.push(UiIntent::ConfirmProfileCreation(input));
                }
                state.list_mode = ListMode::Normal;
                state.wizard = None;
            }
            _ => {}
        },
        KeyCode::Up => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            }
        }
        KeyCode::Down => {
            // Placeholder: would need snapshot length
            state.selected_index += 1;
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Intent derivation helpers
// ---------------------------------------------------------------------------

fn derive_enter_intent(state: &TuiState) -> UiIntent {
    match state.tab_kinds.get(state.active_tab) {
        Some(TabKind::Asset) => UiIntent::UpdateAsset("placeholder".into()),
        _ => UiIntent::RequestReload,
    }
}

fn derive_space_intent(state: &TuiState) -> UiIntent {
    match state.tab_kinds.get(state.active_tab) {
        Some(TabKind::Asset) => UiIntent::InstallAsset("placeholder".into()),
        Some(TabKind::Provider) => UiIntent::ActivateProvider("placeholder".into()),
        Some(TabKind::Vault) => UiIntent::AttachVault("placeholder".into()),
        Some(TabKind::Profile) => {
            // Space on profile tab could launch the selected profile
            UiIntent::StartProfile(crate::domain::profile::ProfileId::new("placeholder"))
        }
        _ => UiIntent::RequestReload,
    }
}

fn is_vault_tab_active(state: &TuiState) -> bool {
    matches!(
        state.tab_kinds.get(state.active_tab),
        Some(crate::app::tab_kind::TabKind::Vault)
    )
}

fn is_mcp_tab_active(state: &TuiState) -> bool {
    matches!(
        state.tab_kinds.get(state.active_tab),
        Some(crate::app::tab_kind::TabKind::Mcp)
    )
}

fn is_profile_tab_active(state: &TuiState) -> bool {
    matches!(
        state.tab_kinds.get(state.active_tab),
        Some(crate::app::tab_kind::TabKind::Profile)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn state_with_tabs(names: Vec<&str>, kinds: Vec<TabKind>) -> TuiState {
        let mut state = TuiState::new(
            names.into_iter().map(|s| s.to_string()).collect(),
            vec![true; kinds.len()],
        );
        state.tab_kinds = kinds;
        state
    }

    #[test]
    fn ctrl_c_quits() {
        let mut state = TuiState::new(vec!["Skills".to_string()], vec![true]);
        let intents = reduce_key(&mut state, ctrl(KeyCode::Char('c')));
        assert_eq!(intents, vec![UiIntent::RequestQuit]);
    }

    #[test]
    fn number_keys_switch_tabs() {
        let mut state = state_with_tabs(
            vec![
                "Skills",
                "MCP",
                "Instructions",
                "Providers",
                "Profiles",
                "Vaults",
            ],
            vec![
                TabKind::Asset,
                TabKind::Mcp,
                TabKind::Asset,
                TabKind::Provider,
                TabKind::Profile,
                TabKind::Vault,
            ],
        );
        let intents = reduce_key(&mut state, key(KeyCode::Char('1')));
        assert_eq!(intents, vec![UiIntent::SwitchTab(0)]);

        let intents = reduce_key(&mut state, key(KeyCode::Char('3')));
        assert_eq!(intents, vec![UiIntent::SwitchTab(2)]);
    }

    #[test]
    fn zero_switches_to_vault_tab() {
        let mut state = state_with_tabs(
            vec!["Skills", "Vaults"],
            vec![TabKind::Asset, TabKind::Vault],
        );
        let intents = reduce_key(&mut state, key(KeyCode::Char('0')));
        assert_eq!(intents, vec![UiIntent::SwitchTab(1)]);
    }

    #[test]
    fn tab_toggles_scope() {
        let mut state = TuiState::new(vec!["Skills".to_string()], vec![true]);
        let intents = reduce_key(&mut state, key(KeyCode::Tab));
        assert!(intents.contains(&UiIntent::ToggleScope));
        assert!(intents.contains(&UiIntent::RequestReload));
        assert_eq!(state.active_scope, crate::domain::scope::Scope::Global);
    }

    #[test]
    fn f2_on_profile_tab_opens_wizard() {
        let mut state = state_with_tabs(
            vec!["Skills", "Profiles"],
            vec![TabKind::Asset, TabKind::Profile],
        );
        state.active_tab = 1;
        let intents = reduce_key(&mut state, key(KeyCode::F(2)));
        assert_eq!(intents, vec![UiIntent::OpenCreateProfileWizard]);
    }

    #[test]
    fn double_esc_quits() {
        let mut state = TuiState::new(vec!["Skills".to_string()], vec![true]);
        // First Esc sets the flag, no quit
        let intents = reduce_key(&mut state, key(KeyCode::Esc));
        assert!(!intents.contains(&UiIntent::RequestQuit));
        assert!(state.esc_pressed_once);

        // Second Esc quits
        let intents = reduce_key(&mut state, key(KeyCode::Esc));
        assert_eq!(intents, vec![UiIntent::RequestQuit]);
    }

    #[test]
    fn up_and_down_emit_navigate() {
        let mut state = TuiState::new(vec!["Skills".to_string()], vec![true]);
        let intents = reduce_key(&mut state, key(KeyCode::Up));
        assert_eq!(intents, vec![UiIntent::NavigateUp]);

        let intents = reduce_key(&mut state, key(KeyCode::Down));
        assert_eq!(intents, vec![UiIntent::NavigateDown]);
    }

    #[test]
    fn search_mode_appends_char() {
        let mut state = TuiState::new(vec!["Skills".to_string()], vec![true]);
        let intents = reduce_key(&mut state, key(KeyCode::Char('a')));
        assert!(intents.contains(&UiIntent::AppendSearchChar('a')));
        assert_eq!(state.search_query, "a");
        assert_eq!(state.list_mode, ListMode::Searching);
    }

    #[test]
    fn space_on_asset_tab_installs() {
        let mut state = state_with_tabs(vec!["Skills"], vec![TabKind::Asset]);
        let intents = reduce_key(&mut state, key(KeyCode::Char(' ')));
        assert!(matches!(&intents[0], UiIntent::InstallAsset(_)));
    }

    #[test]
    fn confirm_delete_profile_modal() {
        let mut state = TuiState::new(vec!["Profiles".to_string()], vec![true]);
        state.list_mode = ListMode::ConfirmDeleteProfile;
        state.pending_delete_profile = Some("my-profile".into());
        let intents = reduce_key(&mut state, key(KeyCode::Enter));
        assert!(intents.iter().any(|i| matches!(
            i,
            UiIntent::DeleteProfile(id) if id.as_str() == "my-profile"
        )));
        assert_eq!(state.list_mode, ListMode::Normal);
    }
}
