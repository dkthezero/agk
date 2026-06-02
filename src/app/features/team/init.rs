use crate::domain::team::TeamConfig;
use anyhow::Result;
use std::path::PathBuf;

pub struct TeamInitResult {
    pub name: String,
    pub created: bool,
    pub message: String,
}

/// Initialize a team configuration at `.agk/team.toml`.
///
/// If the file already exists, returns idempotently with `created: false`.
/// When `dry_run` is true, no files are written.
pub fn team_init(workspace_root: &PathBuf, name: &str, dry_run: bool) -> Result<TeamInitResult> {
    let agk_dir = workspace_root.join(".agk");
    let team_toml_path = agk_dir.join("team.toml");

    if team_toml_path.exists() {
        return Ok(TeamInitResult {
            name: name.to_string(),
            created: false,
            message: "Team configuration already initialized.".to_string(),
        });
    }

    if dry_run {
        return Ok(TeamInitResult {
            name: name.to_string(),
            created: false,
            message: format!("Would initialize team '{}' with team.toml.", name),
        });
    }

    // Create .agk directory if needed
    std::fs::create_dir_all(&agk_dir)?;

    // Write team.toml
    let config = TeamConfig {
        name: name.to_string(),
        source: None,
        branch: Some("main".to_string()),
        vaults: vec![],
        requirements: vec![],
    };
    let content = toml::to_string_pretty(&config)?;
    std::fs::write(&team_toml_path, content)?;

    // Ensure .agk/.gitignore includes config.toml (same pattern as vault init)
    crate::infra::config::gitignore::GitignoreManager::ensure_config_gitignore(workspace_root)?;

    Ok(TeamInitResult {
        name: name.to_string(),
        created: true,
        message: format!("Initialized team '{}' with team.toml.", name),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_team_toml() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let result = team_init(&workspace, "my-team", false).unwrap();

        assert!(result.created);
        assert_eq!(result.name, "my-team");
        assert!(workspace.join(".agk").join("team.toml").exists());
    }

    #[test]
    fn init_writes_valid_config_content() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        team_init(&workspace, "test-team", false).unwrap();

        let team_toml = workspace.join(".agk").join("team.toml");
        let content = std::fs::read_to_string(&team_toml).unwrap();
        let config: TeamConfig = toml::from_str(&content).unwrap();

        assert_eq!(config.name, "test-team");
        assert!(config.source.is_none());
        assert_eq!(config.branch.as_deref(), Some("main"));
        assert!(config.vaults.is_empty());
        assert!(config.requirements.is_empty());
    }

    #[test]
    fn init_dry_run_does_not_create_files() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let result = team_init(&workspace, "dry-team", true).unwrap();

        assert!(!result.created);
        assert!(result.message.contains("Would initialize"));
        assert!(!workspace.join(".agk").join("team.toml").exists());
    }

    #[test]
    fn init_idempotent_returns_not_created() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let result1 = team_init(&workspace, "my-team", false).unwrap();
        assert!(result1.created);

        let result2 = team_init(&workspace, "my-team", false).unwrap();
        assert!(!result2.created);
        assert!(result2.message.contains("already initialized"));
    }

    #[test]
    fn init_creates_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        team_init(&workspace, "gitignore-team", false).unwrap();

        let gitignore = workspace.join(".agk").join(".gitignore");
        assert!(gitignore.exists());
        let content = std::fs::read_to_string(gitignore).unwrap();
        assert!(content.contains("config.toml"));
    }
}