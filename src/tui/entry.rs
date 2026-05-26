use crate::app::ports::ConfigStorePort;
use crate::domain::scope::Scope;
use std::collections::HashMap;

/// Pure composition: build the TUI state from config + registry + scan.
/// No side effects — the caller (main.rs bootstrap) handles the terminal.
pub fn build_state(
    registry: &crate::app::registry::Registry,
    store: &dyn ConfigStorePort,
    scan: crate::app::bootstrap::ScanResult,
    workspace_root: &std::path::Path,
) -> crate::tui::app::AppState {
    let tab_names: Vec<String> = registry
        .feature_sets
        .iter()
        .map(|f| f.display_name().to_string())
        .collect();
    let tab_live: Vec<bool> = registry.feature_sets.iter().map(|f| !f.is_stub()).collect();

    let global_config = store.load(Scope::Global).unwrap_or_default();
    let workspace_config = store.load(Scope::Workspace).unwrap_or_default();
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
    state
}
