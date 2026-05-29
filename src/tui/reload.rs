use crate::app::snapshot::{ProfileEntry, ProviderEntry, VaultEntry};
use crate::domain::asset::ScannedPackage;
use crate::domain::config::ConfigFile;
use crate::domain::scope::Scope;
use std::collections::HashMap;

/// Snapshot produced by a background `reload_state` and sent atomically to the
/// async event loop via `AppEvent::ReloadComplete`.
#[derive(Debug)]
pub struct ReloadSnapshot {
    pub vault_entries: Vec<VaultEntry>,
    pub provider_entries: Vec<ProviderEntry>,
    pub profile_entries: Vec<ProfileEntry>,
    pub packages: HashMap<usize, Vec<ScannedPackage>>,
    pub configs: HashMap<Scope, ConfigFile>,
    pub mcp_state: crate::tui::widgets::mcp::McpState,
}

pub fn apply_reload_snapshot(state: &mut crate::tui::app::AppState, snapshot: ReloadSnapshot) {
    state.vault_entries = snapshot.vault_entries;
    state.provider_entries = snapshot.provider_entries;
    state.profile_entries = snapshot.profile_entries;
    state.packages = snapshot.packages;
    state.configs = snapshot.configs;
    state.mcp_state = snapshot.mcp_state;
}

pub fn compute_reload_snapshot(
    active_scope: Scope,
    ctx: &crate::tui::event::EventContext,
    mcp_state: &mut crate::tui::widgets::mcp::McpState,
) -> ReloadSnapshot {
    mcp_state.refresh();

    let store = &ctx.core.store;
    let registry = &ctx.core.registry;

    let active_config_for_entries = store.load(active_scope).unwrap_or_default();
    let global_config = store.load(Scope::Global).unwrap_or_default();
    let workspace_config = store.load(Scope::Workspace).unwrap_or_default();

    let active_vaults = crate::app::bootstrap::build_vaults(&global_config, &ctx.workspace_root);

    let mut vault_entries = Vec::new();
    let mut provider_entries = Vec::new();
    let mut profile_entries = Vec::new();
    let mut packages = HashMap::new();

    if let Ok(mut scan) = crate::app::bootstrap::scan(registry, &active_vaults) {
        let opt_workspace_config = if active_scope == Scope::Workspace {
            Some(&workspace_config)
        } else {
            None
        };
        crate::app::bootstrap::filter_scan(&mut scan, &global_config, opt_workspace_config);
        vault_entries = crate::app::bootstrap::build_vault_entries(
            &global_config,
            &active_config_for_entries,
            &scan,
            registry,
            &ctx.workspace_root,
        );
        provider_entries =
            crate::app::bootstrap::build_provider_entries(&active_config_for_entries, registry);
        profile_entries = crate::app::bootstrap::build_profile_entries(&active_config_for_entries);
        packages = scan.packages_by_tab.into_iter().enumerate().collect();
    }

    let mut configs = HashMap::new();
    configs.insert(Scope::Global, global_config);
    configs.insert(Scope::Workspace, workspace_config);

    ReloadSnapshot {
        vault_entries,
        provider_entries,
        profile_entries,
        packages,
        configs,
        mcp_state: mcp_state.clone(),
    }
}
