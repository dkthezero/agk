//! Team-aware sync logic.
//!
//! When `team.toml` is present in the workspace, `sync_team_config` runs before
//! the regular asset sync to:
//! - Auto-attach team vaults that aren't yet in `config.toml`
//! - Add team requirement identities to the appropriate vault section buckets
//! - Tag team assets with `AssetSource::Team`
//! - Flag previously-Team assets that are no longer in team requirements

use crate::domain::asset::AssetKind;
use crate::domain::config::{AssetBucket, AssetSource, ConfigFile, VaultConfig};
use crate::domain::team::TeamConfig;

/// Statistics returned by `sync_team_config`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TeamSyncResult {
    pub vaults_attached: Vec<String>,
    pub skills_installed: Vec<String>,
    pub skills_updated: Vec<String>,
    pub skills_removed_from_team: Vec<String>,
    pub errors: Vec<String>,
}

/// Synchronise team configuration into the workspace `ConfigFile`.
///
/// This function works with the `ConfigFile` directly (already loaded by the
/// sync command). It does *not* perform actual asset installation — that is
/// handled by the existing sync flow. What it does:
///
/// 1. For each `TeamVault` in `team_config.vaults`: if the vault ID is not
///    already in `config.vaults`, add it and create a minimal `VaultSection`
///    with a `VaultConfig::Github` entry.
/// 2. For each `TeamRequirement`: add the asset identity to the matching
///    vault-section bucket (skills/instructions) and tag the bucket's source
///    as `AssetSource::Team`.
/// 3. For assets previously tagged `[Team]` that are no longer in the team
///    requirements: add them to `skills_removed_from_team` so the caller can
///    warn the user.
///
/// When `dry_run` is true the function computes the result without mutating
/// `config`.
pub fn sync_team_config(
    team_config: &TeamConfig,
    config: &mut ConfigFile,
    dry_run: bool,
) -> TeamSyncResult {
    let mut result = TeamSyncResult::default();

    // Nothing to do if team config is empty.
    if team_config.name.is_empty() && team_config.vaults.is_empty() {
        return result;
    }

    // -----------------------------------------------------------------------
    // Step 1: Auto-attach team vaults
    // -----------------------------------------------------------------------
    for vault in &team_config.vaults {
        if !config.vaults.contains(&vault.identity) {
            result.vaults_attached.push(vault.identity.clone());
            if !dry_run {
                config.vaults.push(vault.identity.clone());
                let section = config.vault_defs.entry(vault.identity.clone()).or_default();
                if section.vault.is_none() {
                    section.vault = Some(VaultConfig::Github(
                        crate::domain::config::GithubVaultSource {
                            repo: vault
                                .url
                                .trim_start_matches("https://github.com/")
                                .to_string(),
                            r#ref: vault.branch.clone(),
                            path: vault.path.clone().unwrap_or_default(),
                            enterprise_url: None,
                        },
                    ));
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Step 2: Add team requirement identities to vault section buckets
    // -----------------------------------------------------------------------
    for req in &team_config.requirements {
        let identity_str = format!("[{}::0]", req.identity); // placeholder identity

        if dry_run {
            // In dry_run, just check whether the identity already exists
            // without mutating config. Report what would happen.
            if let Some(section) = config.vault_defs.get(&req.vault) {
                let bucket = match req.kind {
                    AssetKind::Skill => &section.skills,
                    AssetKind::Instruction => &section.instructions,
                    AssetKind::McpServer => &section.mcps,
                    AssetKind::Profile => &section.profiles,
                };
                let already_present = bucket
                    .as_ref()
                    .map(|b| {
                        b.items.iter().any(|item| {
                            item.trim_start_matches('[')
                                .split(':')
                                .next()
                                .map(|name| name == req.identity)
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);
                if already_present {
                    result.skills_updated.push(req.identity.clone());
                } else {
                    result.skills_installed.push(req.identity.clone());
                }
            } else {
                result.skills_installed.push(req.identity.clone());
            }
            continue;
        }

        let section = config.vault_defs.entry(req.vault.clone()).or_default();
        let bucket = match req.kind {
            AssetKind::Skill => &mut section.skills,
            AssetKind::Instruction => &mut section.instructions,
            AssetKind::McpServer => &mut section.mcps,
            AssetKind::Profile => &mut section.profiles,
        };

        match bucket {
            Some(ref mut b) => {
                // Check if already present
                let already_present = b.items.iter().any(|item| {
                    item.trim_start_matches('[')
                        .split(':')
                        .next()
                        .map(|name| name == req.identity)
                        .unwrap_or(false)
                });
                if !already_present {
                    b.items.push(identity_str.clone());
                    result.skills_installed.push(req.identity.clone());
                } else {
                    result.skills_updated.push(req.identity.clone());
                }
            }
            None => {
                let new_bucket = AssetBucket {
                    items: vec![identity_str.clone()],
                    source: None,
                };
                *bucket = Some(new_bucket);
                result.skills_installed.push(req.identity.clone());
            }
        }

        // Tag the bucket's source as Team
        if let Some(ref mut b) = bucket {
            b.source = Some(AssetSource::Team);
        }
    }

    // -----------------------------------------------------------------------
    // Step 3: Detect previously-Team assets no longer in requirements
    // -----------------------------------------------------------------------
    let required_ids: Vec<String> = team_config
        .requirements
        .iter()
        .map(|r| r.identity.clone())
        .collect();

    for (vault_id, section) in &config.vault_defs {
        // Only check team vaults
        let is_team_vault = team_config.vaults.iter().any(|v| v.identity == *vault_id);
        if !is_team_vault {
            continue;
        }

        let mut check_bucket = |bucket: &Option<AssetBucket>| {
            if let Some(ref b) = bucket {
                if b.source == Some(AssetSource::Team) {
                    for item in &b.items {
                        let name = item
                            .trim_start_matches('[')
                            .split(':')
                            .next()
                            .unwrap_or(item.as_str());
                        if !required_ids.contains(&name.to_string()) {
                            result.skills_removed_from_team.push(name.to_string());
                        }
                    }
                }
            }
        };
        check_bucket(&section.skills);
        check_bucket(&section.instructions);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{GithubVaultSource, VaultSection};
    use crate::domain::team::{TeamRequirement, TeamVault};

    fn make_team_vault(identity: &str, url: &str, branch: &str) -> TeamVault {
        TeamVault {
            identity: identity.to_string(),
            vault_type: "github".to_string(),
            url: url.to_string(),
            branch: branch.to_string(),
            path: None,
        }
    }

    fn make_team_requirement(identity: &str, vault: &str, kind: AssetKind) -> TeamRequirement {
        TeamRequirement {
            identity: identity.to_string(),
            vault: vault.to_string(),
            kind,
            version_constraint: None,
        }
    }

    // -----------------------------------------------------------------------
    // Test: Empty team config → minimal result
    // -----------------------------------------------------------------------
    #[test]
    fn empty_team_config_returns_minimal_result() {
        let team = TeamConfig::default();
        let mut config = ConfigFile::default();

        let result = sync_team_config(&team, &mut config, false);

        assert!(result.vaults_attached.is_empty());
        assert!(result.skills_installed.is_empty());
        assert!(result.skills_updated.is_empty());
        assert!(result.skills_removed_from_team.is_empty());
        assert!(result.errors.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test: Vaults not in config are auto-attached
    // -----------------------------------------------------------------------
    #[test]
    fn vaults_not_in_config_are_attached() {
        let team = TeamConfig {
            name: "test-team".to_string(),
            source: None,
            branch: Some("main".to_string()),
            vaults: vec![
                make_team_vault("shared", "https://github.com/org/skills", "main"),
                make_team_vault("internal", "https://github.com/org/internal", "develop"),
            ],
            requirements: vec![],
        };
        let mut config = ConfigFile::default();

        let result = sync_team_config(&team, &mut config, false);

        assert_eq!(result.vaults_attached, vec!["shared", "internal"]);
        assert!(config.vaults.contains(&"shared".to_string()));
        assert!(config.vaults.contains(&"internal".to_string()));

        // Vault sections should have Github config
        let shared_section = config.vault_defs.get("shared").unwrap();
        assert!(shared_section.vault.is_some());
        if let Some(VaultConfig::Github(ref src)) = shared_section.vault {
            assert_eq!(src.repo, "org/skills");
            assert_eq!(src.r#ref, "main");
        } else {
            panic!("Expected Github vault config for 'shared'");
        }
    }

    // -----------------------------------------------------------------------
    // Test: Already-attached vaults are not duplicated
    // -----------------------------------------------------------------------
    #[test]
    fn already_attached_vaults_not_duplicated() {
        let team = TeamConfig {
            name: "test-team".to_string(),
            source: None,
            branch: None,
            vaults: vec![make_team_vault(
                "shared",
                "https://github.com/org/skills",
                "main",
            )],
            requirements: vec![],
        };
        let mut config = ConfigFile::default();
        config.vaults.push("shared".to_string());
        let section = VaultSection {
            vault: Some(VaultConfig::Github(GithubVaultSource {
                repo: "org/skills".to_string(),
                r#ref: "main".to_string(),
                path: "skills/".to_string(),
                enterprise_url: None,
            })),
            skills: None,
            instructions: None,
            mcps: None,
            profiles: None,
        };
        config.vault_defs.insert("shared".to_string(), section);

        let result = sync_team_config(&team, &mut config, false);

        assert!(result.vaults_attached.is_empty());
        // Only one entry
        assert_eq!(config.vaults.iter().filter(|v| *v == "shared").count(), 1);
    }

    // -----------------------------------------------------------------------
    // Test: Requirements are tagged as Team
    // -----------------------------------------------------------------------
    #[test]
    fn requirements_are_tagged_as_team() {
        let team = TeamConfig {
            name: "test-team".to_string(),
            source: None,
            branch: Some("main".to_string()),
            vaults: vec![make_team_vault(
                "shared",
                "https://github.com/org/skills",
                "main",
            )],
            requirements: vec![
                make_team_requirement("security-scan", "shared", AssetKind::Skill),
                make_team_requirement("compliance", "shared", AssetKind::Instruction),
            ],
        };
        let mut config = ConfigFile::default();

        let result = sync_team_config(&team, &mut config, false);

        assert_eq!(result.skills_installed.len(), 2);
        assert!(result
            .skills_installed
            .contains(&"security-scan".to_string()));
        assert!(result.skills_installed.contains(&"compliance".to_string()));

        // Check the skills bucket is tagged as Team
        let section = config.vault_defs.get("shared").unwrap();
        let skills_bucket = section.skills.as_ref().unwrap();
        assert_eq!(skills_bucket.source, Some(AssetSource::Team));

        let instructions_bucket = section.instructions.as_ref().unwrap();
        assert_eq!(instructions_bucket.source, Some(AssetSource::Team));
    }

    // -----------------------------------------------------------------------
    // Test: Previously-Team assets no longer in requirements are flagged
    // -----------------------------------------------------------------------
    #[test]
    fn previously_team_assets_no_longer_in_requirements_flagged() {
        let team = TeamConfig {
            name: "test-team".to_string(),
            source: None,
            branch: Some("main".to_string()),
            vaults: vec![make_team_vault(
                "shared",
                "https://github.com/org/skills",
                "main",
            )],
            requirements: vec![
                // Only "security-scan" is required; "old-skill" is no longer listed
                make_team_requirement("security-scan", "shared", AssetKind::Skill),
            ],
        };

        let mut config = ConfigFile::default();
        config.vaults.push("shared".to_string());
        let section = VaultSection {
            vault: Some(VaultConfig::Github(GithubVaultSource {
                repo: "org/skills".to_string(),
                r#ref: "main".to_string(),
                path: String::new(),
                enterprise_url: None,
            })),
            skills: Some(AssetBucket {
                items: vec!["[old-skill:1.0.0:abc123]".to_string()],
                source: Some(AssetSource::Team),
            }),
            instructions: None,
            mcps: None,
            profiles: None,
        };
        config.vault_defs.insert("shared".to_string(), section);

        let result = sync_team_config(&team, &mut config, false);

        assert!(result
            .skills_removed_from_team
            .contains(&"old-skill".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test: Dry run does not mutate config
    // -----------------------------------------------------------------------
    #[test]
    fn dry_run_does_not_mutate_config() {
        let team = TeamConfig {
            name: "test-team".to_string(),
            source: None,
            branch: Some("main".to_string()),
            vaults: vec![make_team_vault(
                "shared",
                "https://github.com/org/skills",
                "main",
            )],
            requirements: vec![make_team_requirement(
                "security-scan",
                "shared",
                AssetKind::Skill,
            )],
        };
        let mut config = ConfigFile::default();
        let config_before = config.clone();

        let result = sync_team_config(&team, &mut config, true);

        // Dry run still reports what would happen
        assert_eq!(result.vaults_attached.len(), 1);
        assert_eq!(result.skills_installed.len(), 1);

        // But config is unchanged
        assert_eq!(config, config_before);
    }

    // -----------------------------------------------------------------------
    // Test: Requirement whose identity already exists is listed as updated
    // -----------------------------------------------------------------------
    #[test]
    fn existing_requirement_listed_as_updated() {
        let team = TeamConfig {
            name: "test-team".to_string(),
            source: None,
            branch: Some("main".to_string()),
            vaults: vec![make_team_vault(
                "shared",
                "https://github.com/org/skills",
                "main",
            )],
            requirements: vec![make_team_requirement(
                "security-scan",
                "shared",
                AssetKind::Skill,
            )],
        };

        let mut config = ConfigFile::default();
        config.vaults.push("shared".to_string());
        let section = VaultSection {
            vault: Some(VaultConfig::Github(GithubVaultSource {
                repo: "org/skills".to_string(),
                r#ref: "main".to_string(),
                path: String::new(),
                enterprise_url: None,
            })),
            skills: Some(AssetBucket {
                items: vec!["[security-scan:1.0.0:abc123]".to_string()],
                source: None,
            }),
            instructions: None,
            mcps: None,
            profiles: None,
        };
        config.vault_defs.insert("shared".to_string(), section);

        let result = sync_team_config(&team, &mut config, false);

        assert!(result.skills_updated.contains(&"security-scan".to_string()));
        assert!(!result
            .skills_installed
            .contains(&"security-scan".to_string()));
    }
}
