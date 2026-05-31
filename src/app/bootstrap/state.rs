use crate::app::registry::Registry;
use crate::app::snapshot::{ProfileEntry, ProviderEntry, VaultEntry};
use crate::app::tab_kind::TabKind;
use crate::domain::config::ConfigFile;

pub fn build_vault_entries(
    global_config: &ConfigFile,
    active_config: &ConfigFile,
    scan: &super::ScanResult,
    registry: &Registry,
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

        let source_path = section
            .and_then(|s| s.vault.as_ref())
            .map(|v| match v {
                crate::domain::config::VaultConfig::Local(local) => {
                    let mut p = std::path::PathBuf::from(&local.path);
                    if p.is_relative() {
                        p = workspace_root.join(p);
                    }
                    p.to_string_lossy().into_owned()
                }
                crate::domain::config::VaultConfig::Github(github) => {
                    format!(
                        "https://github.com/{}/tree/{}/{}",
                        github.repo, github.r#ref, github.path
                    )
                }
                crate::domain::config::VaultConfig::Clawhub(_) => "https://clawhub.ai".to_string(),
            })
            .unwrap_or_default();

        let installed_skills = active_config.installed_skills(&vault_id).len();
        let installed_instructions = active_config.installed_instructions(&vault_id).len();

        let mut available_skills = 0usize;
        let mut available_instructions = 0usize;
        for (tab_idx, pkgs) in scan.packages_by_tab.iter().enumerate() {
            let is_skill = registry
                .feature_sets
                .get(tab_idx)
                .map(|f| f.kind_name() == "skill")
                .unwrap_or(false);
            let is_instruction = registry
                .feature_sets
                .get(tab_idx)
                .map(|f| f.kind_name() == "instruction")
                .unwrap_or(false);
            for pkg in pkgs {
                if pkg.vault_id == vault_id {
                    if is_skill {
                        available_skills += 1;
                    }
                    if is_instruction {
                        available_instructions += 1;
                    }
                }
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
            source_path,
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
            }
        })
        .collect()
}

pub fn build_profile_entries(config: &ConfigFile) -> Vec<ProfileEntry> {
    config
        .profiles
        .iter()
        .cloned()
        .map(|p| ProfileEntry {
            name: p.name,
            provider_id: p.provider_id,
            skills: p.skills,
            mcps: p.mcps,
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
