use crate::domain::scope::Scope;
use std::collections::HashMap;

/// Pure composition: build the TUI state from config + registry + scan.
/// No side effects — the caller (main.rs bootstrap) handles the terminal.
///
/// `global_config` and `workspace_config` are passed in rather than loaded
/// here so `bootstrap::build` and `build_state` do not duplicate disk reads.
pub fn build_state(
    registry: &crate::app::registry::Registry,
    scan: crate::app::bootstrap::ScanResult,
    workspace_root: &std::path::Path,
    global_config: crate::domain::config::ConfigFile,
    workspace_config: crate::domain::config::ConfigFile,
    team_config: Option<crate::domain::team::TeamConfig>,
) -> crate::tui::app::AppState {
    let tab_names: Vec<String> = registry
        .feature_sets
        .iter()
        .map(|f| f.display_name().to_string())
        .collect();
    let tab_live: Vec<bool> = registry.feature_sets.iter().map(|f| !f.is_stub()).collect();

    let active_config_for_entries = workspace_config.clone();

    let vault_entries = crate::app::bootstrap::build_vault_entries(
        &global_config,
        &active_config_for_entries,
        &scan,
        registry,
        workspace_root,
    );
    let provider_entries =
        crate::app::bootstrap::build_provider_entries(&active_config_for_entries, registry);
    let profile_entries = crate::app::bootstrap::build_profile_entries(&active_config_for_entries);
    let tab_kinds = crate::app::bootstrap::build_tab_kinds(registry);

    let packages: HashMap<usize, Vec<_>> = scan.packages_by_tab.into_iter().enumerate().collect();

    let mut state = crate::tui::app::AppState::new(tab_names, tab_live, packages);
    state.tab_kinds = tab_kinds;
    state.vault_entries = vault_entries;
    state.provider_entries = provider_entries;
    state.profile_entries = profile_entries;
    state.configs.insert(Scope::Global, global_config);
    state.configs.insert(Scope::Workspace, workspace_config);
    state.team_config = team_config;
    state
}
