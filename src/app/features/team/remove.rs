use crate::app::ports::TeamConfigStorePort;
use crate::domain::scope::Scope;
use crate::infra::config::team_store::TeamTomlStore;
use anyhow::Result;
use std::path::Path;

pub struct TeamRemoveResult {
    pub identity: String,
    pub removed: bool,
    pub message: String,
}

/// Remove a skill requirement from the team configuration by identity.
///
/// Requirements are keyed by `(identity, vault, kind)`. Removing by identity
/// alone is only safe when the identity is unique across vaults. If no match is
/// found, returns `removed: false` with an informative message; if the identity
/// is ambiguous (present in more than one vault), the removal is refused so we
/// don't delete unintended entries.
pub fn team_remove_requirement(workspace_root: &Path, identity: &str) -> Result<TeamRemoveResult> {
    let store = TeamTomlStore::new(workspace_root.to_path_buf());
    let mut config = store.load(Scope::Workspace)?;

    let matching_vaults: Vec<String> = config
        .requirements
        .iter()
        .filter(|r| r.identity == identity)
        .map(|r| r.vault.clone())
        .collect();

    if matching_vaults.is_empty() {
        return Ok(TeamRemoveResult {
            identity: identity.to_string(),
            removed: false,
            message: format!(
                "Requirement '{}' not found in team configuration.",
                identity
            ),
        });
    }

    if matching_vaults.len() > 1 {
        return Ok(TeamRemoveResult {
            identity: identity.to_string(),
            removed: false,
            message: format!(
                "Requirement '{}' is ambiguous — it exists in {} vaults ({}). \
                 Removing by identity alone is unsafe; disambiguate by vault before removing.",
                identity,
                matching_vaults.len(),
                matching_vaults.join(", ")
            ),
        });
    }

    config.requirements.retain(|r| r.identity != identity);

    store.save(Scope::Workspace, &config)?;

    Ok(TeamRemoveResult {
        identity: identity.to_string(),
        removed: true,
        message: format!(
            "Requirement '{}' removed from team configuration.",
            identity
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::asset::AssetKind;
    use crate::domain::team::{TeamConfig, TeamRequirement, TeamVault};

    fn setup_team_with_requirements(workspace: &Path) -> TeamTomlStore {
        let store = TeamTomlStore::new(workspace.to_path_buf());
        let config = TeamConfig {
            name: "test-team".to_string(),
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
                    version_constraint: Some(">=2.0.0".to_string()),
                },
            ],
        };
        store.save(Scope::Workspace, &config).unwrap();
        store
    }

    #[test]
    fn remove_existing_requirement() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();
        setup_team_with_requirements(&workspace);

        let result = team_remove_requirement(&workspace, "react-conventions").unwrap();
        assert!(result.removed);
        assert!(result.message.contains("removed"));

        // Verify it's gone
        let store = TeamTomlStore::new(workspace);
        let config = store.load(Scope::Workspace).unwrap();
        assert_eq!(config.requirements.len(), 1);
        assert_eq!(config.requirements[0].identity, "security-scan");
    }

    #[test]
    fn remove_nonexistent_requirement() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();
        setup_team_with_requirements(&workspace);

        let result = team_remove_requirement(&workspace, "nonexistent").unwrap();
        assert!(!result.removed);
        assert!(result.message.contains("not found"));
    }

    #[test]
    fn remove_refuses_ambiguous_identity_across_vaults() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();
        let store = TeamTomlStore::new(workspace.clone());
        let config = TeamConfig {
            name: "test-team".to_string(),
            source: None,
            branch: Some("main".to_string()),
            vaults: vec![],
            requirements: vec![
                TeamRequirement {
                    identity: "shared-skill".to_string(),
                    vault: "vault-a".to_string(),
                    kind: AssetKind::Skill,
                    version_constraint: None,
                },
                TeamRequirement {
                    identity: "shared-skill".to_string(),
                    vault: "vault-b".to_string(),
                    kind: AssetKind::Skill,
                    version_constraint: None,
                },
            ],
        };
        store.save(Scope::Workspace, &config).unwrap();

        let result = team_remove_requirement(&workspace, "shared-skill").unwrap();
        assert!(!result.removed);
        assert!(result.message.contains("ambiguous"));

        // Both requirements must still be present — nothing removed.
        let reloaded = store.load(Scope::Workspace).unwrap();
        assert_eq!(reloaded.requirements.len(), 2);
    }

    #[test]
    fn remove_preserves_other_requirements() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();
        setup_team_with_requirements(&workspace);

        team_remove_requirement(&workspace, "react-conventions").unwrap();

        let store = TeamTomlStore::new(workspace);
        let config = store.load(Scope::Workspace).unwrap();
        assert_eq!(config.requirements.len(), 1);
        assert_eq!(config.vaults.len(), 1); // vaults untouched
    }
}
