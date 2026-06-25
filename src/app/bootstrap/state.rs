use crate::app::registry::Registry;
use crate::app::snapshot::{ProfileEntry, ProviderEntry, VaultEntry};
use crate::app::tab_kind::TabKind;
use crate::domain::config::ConfigFile;

pub fn build_vault_entries(
    global_config: &ConfigFile,
    active_config: &ConfigFile,
    scan: &super::ScanResult,
    _registry: &Registry,
    workspace_root: &std::path::Path,
) -> Vec<VaultEntry> {
    let mut entries = Vec::new();
    let mut vault_ids: std::collections::HashSet<String> =
        global_config.vaults.iter().cloned().collect();
    for id in global_config.vault_defs.keys() {
        vault_ids.insert(id.clone());
    }
    let mut sorted_ids: Vec<String> = vault_ids.into_iter().collect();
    sorted_ids.sort();

    for vault_id in sorted_ids {
        let enabled = global_config.vaults.contains(&vault_id);
        let section = global_config.vault_defs.get(&vault_id);
        let kind = section
            .and_then(|s| s.vault.as_ref())
            .map(|v| match v {
                crate::domain::config::VaultConfig::Local(_) => "local",
                crate::domain::config::VaultConfig::Github(_) => "github",
                crate::domain::config::VaultConfig::Clawhub(_) => "clawhub",
            })
            .unwrap_or("local")
            .to_string();

        let (source_path, is_ghes, enterprise_url) = section
            .and_then(|s| s.vault.as_ref())
            .map(|v| match v {
                crate::domain::config::VaultConfig::Local(local) => {
                    let mut p = std::path::PathBuf::from(&local.path);
                    if p.is_relative() {
                        p = workspace_root.join(p);
                    }
                    (p.to_string_lossy().into_owned(), false, None)
                }
                crate::domain::config::VaultConfig::Github(github) => {
                    let ghes = github.enterprise_url.is_some();
                    let url = github.enterprise_url.clone();
                    let display_url = if let Some(ref eu) = github.enterprise_url {
                        format!(
                            "{}/{}/tree/{}/{}",
                            eu.trim_end_matches('/'),
                            github.repo,
                            github.r#ref,
                            github.path
                        )
                    } else {
                        format!(
                            "https://github.com/{}/tree/{}/{}",
                            github.repo, github.r#ref, github.path
                        )
                    };
                    (display_url, ghes, url)
                }
                crate::domain::config::VaultConfig::Clawhub(_) => {
                    ("https://clawhub.ai".to_string(), false, None)
                }
            })
            .unwrap_or_default();

        let installed_skills = active_config.installed_skills(&vault_id).len();
        let installed_instructions = active_config.installed_instructions(&vault_id).len();
        let installed_profiles = active_config.installed_profiles(&vault_id).len();
        let installed_mcps = active_config.installed_mcps(&vault_id).len();

        let mut available_skills = 0usize;
        let mut available_instructions = 0usize;
        let mut available_profiles = 0usize;
        let mut available_mcps = 0usize;
        for pkg in scan.packages_by_tab.iter().flatten() {
            if pkg.vault_id != vault_id {
                continue;
            }
            match pkg.kind {
                crate::domain::asset::AssetKind::Skill => available_skills += 1,
                crate::domain::asset::AssetKind::Instruction => available_instructions += 1,
                crate::domain::asset::AssetKind::Profile => available_profiles += 1,
                crate::domain::asset::AssetKind::McpServer => available_mcps += 1,
            }
        }

        entries.push(VaultEntry {
            id: vault_id.clone(),
            kind,
            enabled,
            installed_skills,
            available_skills,
            installed_instructions,
            available_instructions,
            installed_profiles,
            available_profiles,
            installed_mcps,
            available_mcps,
            source_path,
            is_ghes,
            enterprise_url,
        });
    }
    entries
}

pub fn build_provider_entries(config: &ConfigFile, registry: &Registry) -> Vec<ProviderEntry> {
    registry
        .providers
        .iter()
        .map(|p| {
            let id = p.id().to_string();
            let name = p.name().to_string();
            ProviderEntry {
                id: id.clone(),
                name,
                active: config.providers.contains(&id),
                supports_mcp: p.supports_mcp(),
                supports_profiles: p.supports_profiles(),
                available_tools: p.available_profile_tools(),
                available_permission_modes: p.available_permission_modes(),
            }
        })
        .collect()
}

pub fn build_profile_entries(config: &ConfigFile) -> Vec<ProfileEntry> {
    config
        .profiles
        .iter()
        .cloned()
        .map(|p| {
            // Check if this profile exists in any vault
            let found_in_vault = config.vault_defs.values().any(|section| {
                section
                    .profiles
                    .as_ref()
                    .map(|bucket| {
                        bucket.items.iter().any(|id_str| {
                            crate::domain::config::parse_identity(id_str)
                                .map(|id| id.name == p.name)
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            });

            // If the profile has a vault source, compute drift.
            // If no vault source, it's a local-only profile — no drift.
            let has_drift = if found_in_vault {
                // TODO(v0.4): query VaultPort for exact vault profile refs so we can
                // compute real drift instead of this heuristic. Currently any
                // vault-sourced profile with skills/mcps shows drift, which is
                // overly aggressive — a profile with all matching refs still gets
                // the badge. The `profile diff` CLI command does compute exact
                // drift, but it requires a dedicated call per profile.
                !p.skills.is_empty() || !p.mcps.is_empty()
            } else {
                false
            };

            ProfileEntry {
                name: p.name,
                provider_id: p.provider_id,
                skills: p.skills,
                mcps: p.mcps,
                has_drift,
            }
        })
        .collect()
}

pub fn build_tab_kinds(registry: &Registry) -> Vec<TabKind> {
    registry
        .feature_sets
        .iter()
        .map(|f| match f.kind_name() {
            "vault" => TabKind::Vault,
            "provider" => TabKind::Provider,
            "mcp" => TabKind::Mcp,
            "profile" => TabKind::Profile,
            _ => TabKind::Asset,
        })
        .collect()
}
