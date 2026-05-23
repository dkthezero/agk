//! Composition root: intentionally imports from `infra` to wire concrete adapters
//! into the Registry. This is the one permitted place where `app` depends on `infra`
//! (the "main" side of the hexagonal architecture). All other `app` code must not
//! import from `infra`.

use crate::app::registry::Registry;
use crate::domain::asset::{ProviderEntry, ScannedPackage, VaultEntry};
use crate::domain::config::ConfigFile;
use crate::infra::config::toml_store::TomlConfigStore;
use crate::infra::feature::instruction::InstructionFeatureSet;
use crate::infra::feature::skill::SkillFeatureSet;
use crate::infra::feature::stub::StubFeatureSet;
use crate::infra::vault::local::LocalVaultAdapter;
use crate::tui::app::TabKind;
use anyhow::Result;
use std::path::PathBuf;

pub struct ScanResult {
    /// Index matches `registry.feature_sets` index.
    pub packages_by_tab: Vec<Vec<ScannedPackage>>,
}

/// Build a Registry wired for the given workspace root and run an initial scan.
pub fn build(workspace_root: PathBuf) -> Result<(Registry, ScanResult, TomlConfigStore)> {
    let store = TomlConfigStore::standard(&workspace_root);
    build_with_store(workspace_root, store)
}

pub fn build_with_store(
    workspace_root: PathBuf,
    store: TomlConfigStore,
) -> Result<(Registry, ScanResult, TomlConfigStore)> {
    let mut registry = Registry::new();

    // Feature sets — order defines tab order
    // Data: [0] Skills, [1] MCP, [2] Instructions, [3] Providers, [4] Vault
    // Rendered: [1] Skills [2] MCP [3] Instructions [4] Providers    [0] Vault
    registry.register_feature_set(Box::new(SkillFeatureSet));
    registry.register_feature_set(Box::new(StubFeatureSet::new("mcp", "MCP Servers", "")));
    registry.register_feature_set(Box::new(InstructionFeatureSet));
    registry.register_feature_set(Box::new(StubFeatureSet::new("provider", "Providers", "")));
    registry.register_feature_set(Box::new(StubFeatureSet::new("vault", "Vaults", "")));

    // Extract dynamic vaults from configurations
    let mut global_config =
        crate::app::ports::ConfigStorePort::load(&store, crate::domain::scope::Scope::Global)
            .unwrap_or_default();

    // Ensure ClawHub vault definition is always present (inactive by default)
    if !global_config.vault_defs.contains_key("clawhub") {
        global_config.vault_defs.insert(
            "clawhub".to_string(),
            crate::domain::config::VaultSection {
                vault: Some(crate::domain::config::VaultConfig::Clawhub(
                    crate::domain::config::ClawHubVaultSource {},
                )),
                skills: None,
                instructions: None,
            },
        );
        let _ = crate::app::ports::ConfigStorePort::save(
            &store,
            crate::domain::scope::Scope::Global,
            &global_config,
        );
    }

    let workspace_config =
        crate::app::ports::ConfigStorePort::load(&store, crate::domain::scope::Scope::Workspace)
            .unwrap_or_default();

    // Register all AI Providers
    registry.register_provider(Box::new(
        crate::infra::provider::github::GithubProvider::new(workspace_root.clone()),
    ));
    registry.register_provider(Box::new(
        crate::infra::provider::firebender::FirebenderProvider::new(workspace_root.clone()),
    ));
    registry.register_provider(Box::new(crate::infra::provider::letta::LettaProvider::new(
        workspace_root.clone(),
    )));
    registry.register_provider(Box::new(
        crate::infra::provider::snowflake::SnowflakeProvider::new(workspace_root.clone()),
    ));
    registry.register_provider(Box::new(
        crate::infra::provider::gemini::GeminiProvider::new(workspace_root.clone()),
    ));
    registry.register_provider(Box::new(crate::infra::provider::amp::AmpProvider::new(
        workspace_root.clone(),
    )));
    registry.register_provider(Box::new(
        crate::infra::provider::claude_code::ClaudeCodeProvider::new(workspace_root.clone()),
    ));
    registry.register_provider(Box::new(
        crate::infra::provider::opencode::OpenCodeProvider::new(workspace_root.clone()),
    ));

    // At bootstrap, Active Scope vaults are exclusively mapped from Global config physically.
    let active_vaults = build_vaults(&global_config, &workspace_root);
    for vault in active_vaults {
        registry.register_vault(vault);
    }
    let mut scan_result = scan(&registry, &registry.vaults)?;
    filter_scan(&mut scan_result, &global_config, Some(&workspace_config));

    Ok((registry, scan_result, store))
}

pub fn build_vaults(
    config: &ConfigFile,
    workspace_root: &std::path::Path,
) -> Vec<Box<dyn crate::app::ports::VaultPort>> {
    let mut vaults: Vec<Box<dyn crate::app::ports::VaultPort>> = Vec::new();
    let mut keys: Vec<_> = config.vault_defs.keys().collect();
    keys.sort();

    for vault_id in keys {
        if let Some(section) = config.vault_defs.get(vault_id) {
            if let Some(vault_conf) = &section.vault {
                match vault_conf {
                    crate::domain::config::VaultConfig::Local(local) => {
                        let mut p = std::path::PathBuf::from(&local.path);
                        if p.is_relative() {
                            p = workspace_root.join(p);
                        }
                        vaults.push(Box::new(LocalVaultAdapter::new(vault_id, p)));
                    }
                    crate::domain::config::VaultConfig::Github(github) => {
                        vaults.push(Box::new(
                            crate::infra::vault::github::GithubVaultAdapter::new(
                                vault_id,
                                &github.repo,
                                &github.r#ref,
                                &github.path,
                            ),
                        ));
                    }
                    crate::domain::config::VaultConfig::Clawhub(_) => {
                        vaults.push(Box::new(
                            crate::infra::vault::clawhub::ClawHubVaultAdapter::new(vault_id),
                        ));
                    }
                }
            }
        }
    }
    vaults
}

pub fn filter_scan(
    scan: &mut ScanResult,
    global_config: &ConfigFile,
    workspace_config: Option<&ConfigFile>,
) {
    let mut combined_vaults: std::collections::HashSet<_> =
        global_config.vaults.iter().cloned().collect();
    if let Some(ws) = workspace_config {
        combined_vaults.extend(ws.vaults.iter().cloned());
    }

    for tab_pkgs in &mut scan.packages_by_tab {
        tab_pkgs.retain(|pkg| {
            if combined_vaults.contains(&pkg.vault_id) {
                true
            } else {
                let is_global = match pkg.kind {
                    crate::domain::asset::AssetKind::Skill => {
                        global_config.is_skill_installed(&pkg.vault_id, &pkg.identity.name)
                    }
                    crate::domain::asset::AssetKind::Instruction => {
                        global_config.is_instruction_installed(&pkg.vault_id, &pkg.identity.name)
                    }
                    crate::domain::asset::AssetKind::McpServer => false,
                };
                let is_ws = if let Some(ws) = workspace_config {
                    match pkg.kind {
                        crate::domain::asset::AssetKind::Skill => {
                            ws.is_skill_installed(&pkg.vault_id, &pkg.identity.name)
                        }
                        crate::domain::asset::AssetKind::Instruction => {
                            ws.is_instruction_installed(&pkg.vault_id, &pkg.identity.name)
                        }
                        crate::domain::asset::AssetKind::McpServer => false,
                    }
                } else {
                    false
                };
                is_global || is_ws
            }
        });
    }
}

/// Scan all vaults for all feature sets and return packages grouped by tab index.
pub fn scan(
    registry: &Registry,
    vaults: &[Box<dyn crate::app::ports::VaultPort>],
) -> Result<ScanResult> {
    let mut packages_by_tab = Vec::new();
    for feature in &registry.feature_sets {
        let mut tab_packages = Vec::new();
        if !feature.is_stub() {
            for vault in vaults {
                match vault.list_packages(feature.as_ref()) {
                    Ok(mut pkgs) => tab_packages.append(&mut pkgs),
                    Err(e) => eprintln!("vault '{}' scan error: {}", vault.id(), e),
                }
            }
        }
        packages_by_tab.push(tab_packages);
    }
    Ok(ScanResult { packages_by_tab })
}

pub fn build_vault_entries(
    global_config: &ConfigFile,
    active_config: &ConfigFile,
    scan: &ScanResult,
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
            _ => TabKind::Asset,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(root: &std::path::Path, name: &str) {
        let skill_dir = root.join("skills").join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), format!("# {}", name)).unwrap();
    }

    #[test]
    fn bootstrap_produces_five_tabs() {
        let dir = tempfile::tempdir().unwrap();
        let (registry, _, _store) = build(dir.path().to_path_buf()).unwrap();
        assert_eq!(registry.feature_sets.len(), 5);
    }

    #[test]
    fn bootstrap_scans_skills() {
        let dir = tempfile::tempdir().unwrap();
        let workspace_root = dir.path().to_path_buf();
        let agk_dir = workspace_root.join(".agk");
        std::fs::create_dir_all(&agk_dir).unwrap();
        let global_dir = dir.path().join("global");
        std::fs::create_dir_all(&global_dir).unwrap();
        let config_content = format!(
            r#"
version = 1
vaults = ["workspace"]
[workspace.vault]
type = "local"
path = "{}"
"#,
            workspace_root.display()
        );
        std::fs::write(global_dir.join("config.toml"), config_content).unwrap();

        make_skill(dir.path(), "alpha");
        make_skill(dir.path(), "beta");
        let store =
            TomlConfigStore::new(global_dir.join("config.toml"), agk_dir.join("config.toml"));
        let (_, scan, _store) = build_with_store(workspace_root, store).unwrap();
        // Skills is tab index 0 (first tab, numbered [1])
        let ws_skills = scan.packages_by_tab[0]
            .iter()
            .filter(|p| p.vault_id == "workspace")
            .count();
        assert_eq!(ws_skills, 2);
    }

    #[test]
    fn bootstrap_skill_tab_is_live() {
        let dir = tempfile::tempdir().unwrap();
        let (registry, _, _store) = build(dir.path().to_path_buf()).unwrap();
        // Skills is tab index 0 (first tab, numbered [1])
        assert!(!registry.feature_sets[0].is_stub());
    }

    #[test]
    fn bootstrap_scans_instructions() {
        let dir = tempfile::tempdir().unwrap();
        let workspace_root = dir.path().to_path_buf();
        let agk_dir = workspace_root.join(".agk");
        std::fs::create_dir_all(&agk_dir).unwrap();
        let global_dir = dir.path().join("global");
        std::fs::create_dir_all(&global_dir).unwrap();
        let config_content = format!(
            r#"
version = 1
vaults = ["workspace"]
[workspace.vault]
type = "local"
path = "{}"
"#,
            workspace_root.display()
        );
        std::fs::write(global_dir.join("config.toml"), config_content).unwrap();

        let inst_dir = dir.path().join("instructions").join("my-instruction");
        std::fs::create_dir_all(&inst_dir).unwrap();
        std::fs::write(inst_dir.join("AGENTS.md"), "# My Instruction").unwrap();
        let store =
            TomlConfigStore::new(global_dir.join("config.toml"), agk_dir.join("config.toml"));
        let (_, scan, _store) = build_with_store(workspace_root, store).unwrap();
        // Instructions is tab index 2 (0=Skills, 1=MCP, 2=Instructions)
        let ws_insts: Vec<_> = scan.packages_by_tab[2]
            .iter()
            .filter(|p| p.vault_id == "workspace")
            .collect();
        assert_eq!(ws_insts.len(), 1);
        assert_eq!(ws_insts[0].identity.name, "my-instruction");
    }

    #[test]
    fn bootstrap_instructions_tab_is_live() {
        let dir = tempfile::tempdir().unwrap();
        let (registry, _, _store) = build(dir.path().to_path_buf()).unwrap();
        // Instructions is tab index 2 (0=Skills, 1=MCP, 2=Instructions)
        assert!(!registry.feature_sets[2].is_stub());
    }

    #[test]
    fn bootstrap_includes_clawhub_vault_entry() {
        let dir = tempfile::tempdir().unwrap();
        let workspace_root = dir.path().to_path_buf();
        let agk_dir = workspace_root.join(".agk");
        std::fs::create_dir_all(&agk_dir).unwrap();
        let global_dir = dir.path().join("global");
        std::fs::create_dir_all(&global_dir).unwrap();
        let config_content = r#"
version = 1
vaults = []

[clawhub.vault]
type = "clawhub"
"#;
        std::fs::write(global_dir.join("config.toml"), config_content).unwrap();
        let store =
            TomlConfigStore::new(global_dir.join("config.toml"), agk_dir.join("config.toml"));
        let (registry, _scan, _store) = build_with_store(workspace_root, store).unwrap();
        assert!(registry.vaults.iter().any(|v| v.id() == "clawhub"));
    }

    #[test]
    fn bootstrap_clawhub_vault_inactive_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let workspace_root = dir.path().to_path_buf();
        let agk_dir = workspace_root.join(".agk");
        std::fs::create_dir_all(&agk_dir).unwrap();
        let global_dir = dir.path().join("global");
        std::fs::create_dir_all(&global_dir).unwrap();
        let config_content = "version = 1\nvaults = []\n";
        std::fs::write(global_dir.join("config.toml"), config_content).unwrap();
        let store =
            TomlConfigStore::new(global_dir.join("config.toml"), agk_dir.join("config.toml"));
        let (registry, _scan, store) = build_with_store(workspace_root, store).unwrap();
        let global_config =
            crate::app::ports::ConfigStorePort::load(&store, crate::domain::scope::Scope::Global)
                .unwrap();
        let entries = build_vault_entries(
            &global_config,
            &global_config,
            &_scan,
            &registry,
            std::path::Path::new("."),
        );
        let clawhub_entry = entries.iter().find(|e| e.id == "clawhub");
        assert!(
            clawhub_entry.is_some(),
            "ClawHub should appear in vault entries"
        );
        assert!(
            !clawhub_entry.unwrap().enabled,
            "ClawHub should be inactive by default"
        );
    }
}
