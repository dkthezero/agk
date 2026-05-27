use crate::app::registry::Registry;
use crate::domain::asset::ScannedPackage;
use crate::domain::config::ConfigFile;
use anyhow::Result;

pub struct ScanResult {
    pub packages_by_tab: Vec<Vec<ScannedPackage>>,
}

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
                        vaults.push(Box::new(
                            crate::infra::vault::local::LocalVaultAdapter::new(vault_id, p),
                        ));
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
