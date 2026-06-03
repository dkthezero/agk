use crate::app::features::common::parse_identity_from_item;
use crate::app::ports::ConfigStorePort;
use crate::app::ports::TeamConfigStorePort;
use crate::domain::scope::Scope;
use anyhow::Result;

pub struct TeamStatusResult {
    pub team_name: String,
    pub installed: usize,
    pub required: usize,
    pub personal: usize,
}

impl TeamStatusResult {
    pub fn summary(&self) -> String {
        format!(
            "Team '{}' status: {}/{} requirements installed, {} personal assets",
            self.team_name, self.installed, self.required, self.personal
        )
    }
}

/// Count how many team requirements are installed vs total, and how many
/// personal (non-team) assets are present.
pub fn team_status(
    team_store: &dyn TeamConfigStorePort,
    config_store: &dyn ConfigStorePort,
) -> Result<TeamStatusResult> {
    let team_config = team_store.load(Scope::Workspace)?;

    if team_config.name.is_empty() {
        return Ok(TeamStatusResult {
            team_name: "(no team)".to_string(),
            installed: 0,
            required: 0,
            personal: 0,
        });
    }

    let total_required = team_config.requirements.len();

    // Load installed config to count matching requirements
    let installed_config = config_store.load(Scope::Workspace).unwrap_or_default();

    let team_vault_ids: Vec<String> = team_config
        .vaults
        .iter()
        .map(|v| v.identity.clone())
        .collect();

    let mut installed_count = 0;
    for req in &team_config.requirements {
        if is_requirement_installed(&installed_config, &req.identity, &team_vault_ids) {
            installed_count += 1;
        }
    }

    // Count personal assets (installed assets NOT from team vaults)
    let personal_count = count_personal_assets(&installed_config, &team_vault_ids);

    Ok(TeamStatusResult {
        team_name: team_config.name.clone(),
        installed: installed_count,
        required: total_required,
        personal: personal_count,
    })
}

fn is_requirement_installed(
    config: &crate::domain::config::ConfigFile,
    identity: &str,
    vault_ids: &[String],
) -> bool {
    for vault_id in vault_ids {
        if let Some(section) = config.vault_defs.get(vault_id) {
            if let Some(ref bucket) = section.skills {
                if bucket
                    .items
                    .iter()
                    .any(|item| parse_identity_from_item(item).as_deref() == Some(identity))
                {
                    return true;
                }
            }
            if let Some(ref bucket) = section.instructions {
                if bucket
                    .items
                    .iter()
                    .any(|item| parse_identity_from_item(item).as_deref() == Some(identity))
                {
                    return true;
                }
            }
            if let Some(ref bucket) = section.mcps {
                if bucket
                    .items
                    .iter()
                    .any(|item| parse_identity_from_item(item).as_deref() == Some(identity))
                {
                    return true;
                }
            }
            if let Some(ref bucket) = section.profiles {
                if bucket
                    .items
                    .iter()
                    .any(|item| parse_identity_from_item(item).as_deref() == Some(identity))
                {
                    return true;
                }
            }
        }
    }
    false
}

fn count_personal_assets(
    config: &crate::domain::config::ConfigFile,
    team_vault_ids: &[String],
) -> usize {
    let mut count = 0;
    for (vault_id, section) in &config.vault_defs {
        if team_vault_ids.contains(vault_id) {
            continue; // skip team vaults
        }
        if let Some(ref bucket) = section.skills {
            count += bucket.items.len();
        }
        if let Some(ref bucket) = section.instructions {
            count += bucket.items.len();
        }
        if let Some(ref bucket) = section.mcps {
            count += bucket.items.len();
        }
        if let Some(ref bucket) = section.profiles {
            count += bucket.items.len();
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::{FakeStore, FakeTeamConfigStore};
    use crate::domain::asset::AssetKind;
    use crate::domain::team::{TeamConfig, TeamRequirement, TeamVault};

    #[test]
    fn status_no_team_config() {
        let team_store = FakeTeamConfigStore::new();
        let config_store = FakeStore::new();

        let result = team_status(&team_store, &config_store).unwrap();
        assert_eq!(result.team_name, "(no team)");
        assert_eq!(result.installed, 0);
        assert_eq!(result.required, 0);
    }

    #[test]
    fn status_with_team() {
        let team_store = FakeTeamConfigStore::new();
        let config = TeamConfig {
            name: "my-team".to_string(),
            source: None,
            branch: Some("main".to_string()),
            vaults: vec![TeamVault {
                identity: "shared".to_string(),
                vault_type: "github".to_string(),
                url: "https://github.com/org/skills".to_string(),
                branch: "main".to_string(),
                path: None,
            }],
            requirements: vec![
                TeamRequirement {
                    identity: "react-conventions".to_string(),
                    vault: "shared".to_string(),
                    kind: AssetKind::Skill,
                    version_constraint: None,
                },
                TeamRequirement {
                    identity: "security-scan".to_string(),
                    vault: "shared".to_string(),
                    kind: AssetKind::Instruction,
                    version_constraint: None,
                },
            ],
        };
        team_store.save(Scope::Workspace, &config).unwrap();

        let config_store = FakeStore::new();
        let result = team_status(&team_store, &config_store).unwrap();
        assert_eq!(result.team_name, "my-team");
        assert_eq!(result.required, 2);
        assert_eq!(result.installed, 0); // nothing installed
    }

    #[test]
    fn status_summary_format() {
        let result = TeamStatusResult {
            team_name: "my-team".to_string(),
            installed: 1,
            required: 3,
            personal: 5,
        };
        assert!(result.summary().contains("1/3 requirements installed"));
        assert!(result.summary().contains("5 personal assets"));
    }
}
