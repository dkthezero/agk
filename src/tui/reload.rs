use crate::app::snapshot::{
    DiscoveredMcp, DiscoveredProfile, ProfileEntry, ProviderEntry, VaultEntry,
};
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
    pub discovered_profiles: Vec<DiscoveredProfile>,
    pub discovered_mcps: Vec<DiscoveredMcp>,
    pub packages: HashMap<usize, Vec<ScannedPackage>>,
    pub configs: HashMap<Scope, ConfigFile>,
    pub mcp_state: crate::tui::widgets::mcp::McpState,
    pub team_config: Option<crate::domain::team::TeamConfig>,
}

pub fn apply_reload_snapshot(state: &mut crate::tui::app::AppState, snapshot: ReloadSnapshot) {
    state.vault_entries = snapshot.vault_entries;
    state.provider_entries = snapshot.provider_entries;
    state.profile_entries = snapshot.profile_entries;
    state.discovered_profiles = snapshot.discovered_profiles;
    state.discovered_mcps = snapshot.discovered_mcps;
    state.packages = snapshot.packages;
    state.configs = snapshot.configs;
    state.mcp_state = snapshot.mcp_state;
    state.team_config = snapshot.team_config;
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
    let mut discovered_profiles = Vec::new();
    let mut discovered_mcps = Vec::new();
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

        // Build discovered profiles: profile packages not yet in config.profiles
        let registered_profile_names: std::collections::HashSet<&str> = active_config_for_entries
            .profiles
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        for (_, pkgs) in packages.iter() {
            for pkg in pkgs {
                if pkg.kind == crate::domain::asset::AssetKind::Profile
                    && !registered_profile_names.contains(pkg.identity.name.as_str())
                {
                    discovered_profiles.push(DiscoveredProfile {
                        name: pkg.identity.name.clone(),
                        vault_id: pkg.vault_id.clone(),
                        description: pkg.description.clone(),
                    });
                }
            }
        }

        // Build discovered MCPs: MCP packages not yet registered
        for (_, pkgs) in packages.iter() {
            for pkg in pkgs {
                if pkg.kind == crate::domain::asset::AssetKind::McpServer {
                    discovered_mcps.push(DiscoveredMcp {
                        name: pkg.identity.name.clone(),
                        vault_id: pkg.vault_id.clone(),
                        description: pkg.description.clone(),
                    });
                }
            }
        }
    }

    // Filter discovered MCPs: remove ones already registered
    mcp_state.refresh_with_discovered(discovered_mcps);
    let filtered_discovered_mcps = mcp_state.discovered.clone();

    let mut configs = HashMap::new();
    configs.insert(Scope::Global, global_config);
    configs.insert(Scope::Workspace, workspace_config);

    // Reload team config for accurate status bar
    let team_config = ctx
        .core
        .team_config_store
        .load(Scope::Workspace)
        .ok()
        .filter(|c| !c.name.is_empty());

    ReloadSnapshot {
        vault_entries,
        provider_entries,
        profile_entries,
        discovered_profiles,
        discovered_mcps: filtered_discovered_mcps,
        packages,
        configs,
        mcp_state: mcp_state.clone(),
        team_config,
    }
}
