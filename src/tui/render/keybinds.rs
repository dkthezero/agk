use crate::app::ports::WizardStep;
use crate::app::tab_kind::TabKind;
use crate::tui::app::AppState;
use crate::tui::list_mode::ListMode;

pub fn resolve_keybinds(state: &AppState) -> &'static str {
    let active_kind = state
        .tab_kinds
        .get(state.active_tab)
        .cloned()
        .unwrap_or(TabKind::Asset);

    if matches!(state.list_mode, ListMode::SelectProviderRoot { .. }) {
        "[↑/↓] Move  [Enter] Confirm  [Esc] Cancel"
    } else if state.is_attach_vault_mode() || state.is_register_mcp_mode() {
        "[Enter] Confirm  [Esc] Cancel"
    } else if matches!(
        state.list_mode,
        ListMode::ConfirmMcpTest
            | ListMode::ConfirmClawHubInstall
            | ListMode::ConfirmDetachVault
            | ListMode::ConfirmDeactivateLastProvider
            | ListMode::ConfirmDeleteProfile
    ) {
        ""
    } else if state.is_profile_wizard_mode() {
        if let Some(ref ws) = state.wizard_state {
            match ws.steps.get(ws.step_index) {
                Some(WizardStep::Checklist { .. }) => "[Space] Toggle  [Enter] Confirm  [Esc] Back",
                Some(WizardStep::Review { .. }) => "[Enter] Confirm Create  [Esc] Back",
                _ => "[Enter] Confirm  [Esc] Cancel",
            }
        } else {
            ""
        }
    } else {
        match active_kind {
            TabKind::Asset => {
                "[↑/↓] Move  [Space] Toggle  [Enter] Update  [F5] Update All  [F4] Refresh  [Ctrl+O] Open Folder  [Ctrl+T] Terminal  [type] Search  [Esc]x2 Quit"
            }
            TabKind::Provider => {
                "[↑/↓] Move  [Space] Toggle  [Enter] Update  [F4] Refresh  [Esc]x2 Quit"
            }
            TabKind::Mcp => {
                "[↑/↓] Move  [F2] Add MCP  [Space] Enable  [Enter] Test  [Esc]x2 Quit"
            }
            TabKind::Vault => {
                "[↑/↓] Move  [F2] Attach New  [Space] Toggle  [F4] Refresh  [Esc]x2 Quit"
            }
            TabKind::Profile => {
                "[↑/↓] Move  [F2] Add Profile  [Delete] Remove  [Esc]x2 Quit"
            }
            TabKind::Analytics => "",
        }
    }
}
