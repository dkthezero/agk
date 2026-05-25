use crate::app::tab_kind::TabKind;
use crate::domain::scope::Scope;
use std::collections::HashMap;

/// Pure UI navigation state.  Contains **no domain data** (no ConfigFile,
/// no ScannedPackage, no ProviderPort references).  All business data lives
/// in [`crate::app::event::WorkspaceSnapshot`] and is applied to the
/// presenter layer separately.
///
/// This struct is designed to be testable without a terminal, filesystem,
/// or async runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct TuiState {
    pub active_tab: usize,
    pub search_query: String,
    pub selected_index: usize,
    pub list_mode: ListMode,
    pub status_line: String,
    pub tab_names: Vec<String>,
    pub tab_live: Vec<bool>,
    pub tab_kinds: Vec<TabKind>,
    pub active_scope: Scope,
    pub esc_pressed_once: bool,
    pub scroll_offset: usize,
    pub scroll_tick: u8,
    // Wizard / modal state
    pub wizard: Option<WizardState>,
    // Modal pending fields
    pub pending_vault_id: String,
    pub pending_vault_local_path: String,
    pub pending_delete_profile: Option<String>,
}

impl TuiState {
    pub fn new(tab_names: Vec<String>, tab_live: Vec<bool>) -> Self {
        Self {
            active_tab: 0,
            search_query: String::new(),
            selected_index: 0,
            list_mode: ListMode::Normal,
            status_line: String::new(),
            tab_names,
            tab_live,
            tab_kinds: Vec::new(),
            active_scope: Scope::Workspace,
            esc_pressed_once: false,
            scroll_offset: 0,
            scroll_tick: 0,
            wizard: None,
            pending_vault_id: String::new(),
            pending_vault_local_path: String::new(),
            pending_delete_profile: None,
        }
    }

    pub fn list_length(&self) -> usize {
        // Placeholder: in full implementation this reads from snapshot
        0
    }

    pub fn is_active_tab_live(&self) -> bool {
        match self.tab_kinds.get(self.active_tab) {
            Some(TabKind::Vault) | Some(TabKind::Provider) => true,
            _ => self.tab_live.get(self.active_tab).copied().unwrap_or(false),
        }
    }

    pub fn toggle_scope(&mut self) {
        self.active_scope = match self.active_scope {
            Scope::Global => Scope::Workspace,
            Scope::Workspace => Scope::Global,
        };
    }

    pub fn scope_label(&self) -> &'static str {
        match self.active_scope {
            Scope::Global => "[Tab] GLOBAL",
            Scope::Workspace => "[Tab] WORKSPACE",
        }
    }
}

/// Simplified list mode for the pure reducer.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ListMode {
    Normal,
    Searching,
    AttachVault,
    AttachVaultBranch,
    AttachVaultPath,
    AttachVaultName,
    ConfirmDetachVault,
    ConfirmClawHubInstall,
    ConfirmDeactivateLastProvider,
    RegisterMcpStepName,
    RegisterMcpStepCommand,
    RegisterMcpStepArgs,
    RegisterMcpStepTransport,
    RegisterMcpStepDescription,
    ConfirmMcpTest,
    SelectProviderRoot {
        provider_id: String,
        options: Vec<(String, String)>,
        selected: usize,
    },
    ProfileWizard,
    ConfirmDeleteProfile,
}

/// Lightweight wizard accumulator tracked purely in UI state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WizardState {
    pub provider_id: String,
    pub profile_name: String,
    pub description: String,
    pub selected_skills: Vec<String>,
    pub selected_mcps: Vec<String>,
    pub step_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_starts_workspace() {
        let state = TuiState::new(vec!["Skills".into()], vec![true]);
        assert_eq!(state.active_scope, Scope::Workspace);
    }

    #[test]
    fn toggle_scope_switches() {
        let mut state = TuiState::new(vec![], vec![]);
        state.toggle_scope();
        assert_eq!(state.active_scope, Scope::Global);
    }

    #[test]
    fn active_tab_starts_zero() {
        let state = TuiState::new(vec!["Skills".into()], vec![true]);
        assert_eq!(state.active_tab, 0);
    }
}
