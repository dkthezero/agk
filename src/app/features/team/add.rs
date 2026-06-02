use crate::app::ports::TeamConfigStorePort;
use crate::domain::asset::AssetKind;
use crate::domain::scope::Scope;
use crate::domain::team::{TeamConfig, TeamRequirement, TeamVault};
use crate::infra::config::team_store::TeamTomlStore;
use anyhow::Result;
use std::path::PathBuf;

pub struct TeamAddVaultResult {
    pub identity: String,
    pub message: String,
}

/// Add a vault entry to the team configuration.
///
/// Creates `team.toml` with defaults if it does not exist yet.
pub fn team_add_vault(
    workspace_root: &PathBuf,
    identity: &str,
    vault_type: &str,
    url: &str,
    branch: &str,
) -> Result<TeamAddVaultResult> {
    let store = TeamTomlStore::new(workspace_root.clone());
    let mut config = ensure_config(&store);

    // Check for duplicate
    if config.vaults.iter().any(|v| v.identity == identity) {
        return Ok(TeamAddVaultResult {
            identity: identity.to_string(),
            message: format!("Vault '{}' already exists in team configuration.", identity),
        });
    }

    let vault = TeamVault {
        identity: identity.to_string(),
        vault_type: vault_type.to_string(),
        url: url.to_string(),
        branch: branch.to_string(),
        path: None,
    };
    config.vaults.push(vault);
    store.save(Scope::Workspace, &config)?;

    Ok(TeamAddVaultResult {
        identity: identity.to_string(),
        message: format!("Vault '{}' added to team configuration.", identity),
    })
}

pub struct TeamAddRequirementResult {
    pub identity: String,
    pub message: String,
}

/// Add a skill requirement to the team configuration.
///
/// Creates `team.toml` with defaults if it does not exist yet.
pub fn team_add_requirement(
    workspace_root: &PathBuf,
    identity: &str,
    vault: &str,
    kind: &str,
    version_constraint: Option<&str>,
) -> Result<TeamAddRequirementResult> {
    let store = TeamTomlStore::new(workspace_root.clone());
    let mut config = ensure_config(&store);

    // Parse asset kind
    let asset_kind = match kind {
        "skill" => AssetKind::Skill,
        "instruction" => AssetKind::Instruction,
        "mcp" => AssetKind::McpServer,
        "profile" => AssetKind::Profile,
        _ => AssetKind::Skill, // default fallback
    };

    // Check for duplicate
    if config
        .requirements
        .iter()
        .any(|r| r.identity == identity && r.vault == vault)
    {
        return Ok(TeamAddRequirementResult {
            identity: identity.to_string(),
            message: format!(
                "Requirement '{}' from vault '{}' already exists in team configuration.",
                identity, vault
            ),
        });
    }

    let requirement = TeamRequirement {
        identity: identity.to_string(),
        vault: vault.to_string(),
        kind: asset_kind,
        version_constraint: version_constraint.map(|s| s.to_string()),
    };
    config.requirements.push(requirement);
    store.save(Scope::Workspace, &config)?;

    Ok(TeamAddRequirementResult {
        identity: identity.to_string(),
        message: format!("Requirement '{}' added to team configuration.", identity),
    })
}

/// Load the team config or create a default one if it does not exist yet.
fn ensure_config(store: &TeamTomlStore) -> TeamConfig {
    match store.load(Scope::Workspace) {
        Ok(config) if !config.name.is_empty() => config,
        _ => TeamConfig::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_vault_creates_entry() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        // Initialize team first
        crate::app::features::team::init::team_init(&workspace, "test-team", false).unwrap();

        let result = team_add_vault(&workspace, "shared", "github", "https://github.com/org/skills", "main").unwrap();
        assert!(result.message.contains("added"));

        // Reload and verify
        let store = TeamTomlStore::new(workspace);
        let config = store.load(Scope::Workspace).unwrap();
        assert_eq!(config.vaults.len(), 1);
        assert_eq!(config.vaults[0].identity, "shared");
    }

    #[test]
    fn add_vault_duplicate_returns_message() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        crate::app::features::team::init::team_init(&workspace, "test-team", false).unwrap();
        team_add_vault(&workspace, "shared", "github", "https://github.com/org/skills", "main").unwrap();

        let result = team_add_vault(&workspace, "shared", "github", "https://github.com/org/skills", "main").unwrap();
        assert!(result.message.contains("already exists"));
    }

    #[test]
    fn add_requirement_creates_entry() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        crate::app::features::team::init::team_init(&workspace, "test-team", false).unwrap();
        team_add_vault(&workspace, "shared", "github", "https://github.com/org/skills", "main").unwrap();

        let result = team_add_requirement(&workspace, "react-conventions", "shared", "skill", None).unwrap();
        assert!(result.message.contains("added"));

        // Reload and verify
        let store = TeamTomlStore::new(workspace);
        let config = store.load(Scope::Workspace).unwrap();
        assert_eq!(config.requirements.len(), 1);
        assert_eq!(config.requirements[0].identity, "react-conventions");
        assert_eq!(config.requirements[0].vault, "shared");
        assert_eq!(config.requirements[0].kind, AssetKind::Skill);
    }

    #[test]
    fn add_requirement_with_version_constraint() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        crate::app::features::team::init::team_init(&workspace, "test-team", false).unwrap();
        team_add_vault(&workspace, "shared", "github", "https://github.com/org/skills", "main").unwrap();

        team_add_requirement(&workspace, "security-scan", "shared", "instruction", Some(">=2.0.0")).unwrap();

        let store = TeamTomlStore::new(workspace);
        let config = store.load(Scope::Workspace).unwrap();
        assert_eq!(config.requirements[0].version_constraint.as_deref(), Some(">=2.0.0"));
    }

    #[test]
    fn add_requirement_duplicate_returns_message() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        crate::app::features::team::init::team_init(&workspace, "test-team", false).unwrap();
        team_add_vault(&workspace, "shared", "github", "https://github.com/org/skills", "main").unwrap();
        team_add_requirement(&workspace, "react-conventions", "shared", "skill", None).unwrap();

        let result = team_add_requirement(&workspace, "react-conventions", "shared", "skill", None).unwrap();
        assert!(result.message.contains("already exists"));
    }
}