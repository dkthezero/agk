use crate::app::ports::TeamConfigStorePort;
use crate::domain::scope::Scope;
use crate::domain::team::TeamConfig;
use anyhow::Result;
use std::path::PathBuf;

pub struct TeamTomlStore {
    workspace_root: PathBuf,
    lock: std::sync::Mutex<()>,
}

impl TeamTomlStore {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            lock: std::sync::Mutex::new(()),
        }
    }

    fn team_toml_path(&self, scope: &Scope) -> PathBuf {
        match scope {
            Scope::Workspace => self.workspace_root.join(".agk").join("team.toml"),
            Scope::Global => crate::domain::paths::global_config_root().join("team.toml"),
        }
    }
}

impl TeamConfigStorePort for TeamTomlStore {
    fn load(&self, scope: Scope) -> Result<TeamConfig> {
        let _guard = self
            .lock
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let path = self.team_toml_path(&scope);
        if !path.exists() {
            return Ok(TeamConfig::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let config: TeamConfig = toml::from_str(&content)?;
        Ok(config)
    }

    fn save(&self, scope: Scope, config: &TeamConfig) -> Result<()> {
        let _guard = self
            .lock
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let path = self.team_toml_path(&scope);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn exists(&self, scope: Scope) -> bool {
        self.team_toml_path(&scope).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::asset::AssetKind;

    fn make_store(dir: &std::path::Path) -> TeamTomlStore {
        TeamTomlStore::new(dir.to_path_buf())
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());
        let config = store.load(Scope::Workspace).unwrap();
        assert_eq!(config, TeamConfig::default());
    }

    #[test]
    fn round_trip_minimal_config() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());
        let config = TeamConfig {
            name: "my-team".to_string(),
            ..TeamConfig::default()
        };
        store.save(Scope::Workspace, &config).unwrap();
        let loaded = store.load(Scope::Workspace).unwrap();
        assert_eq!(loaded, config);
    }

    #[test]
    fn round_trip_full_config() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());
        let config = TeamConfig {
            name: "enterprise-team".to_string(),
            source: Some("https://github.com/acme/team-config".to_string()),
            branch: Some("develop".to_string()),
            vaults: vec![crate::domain::team::TeamVault {
                identity: "shared-vault".to_string(),
                vault_type: "github".to_string(),
                url: "https://github.com/acme/shared-skills".to_string(),
                branch: "release".to_string(),
                path: Some("skills/".to_string()),
            }],
            requirements: vec![crate::domain::team::TeamRequirement {
                identity: "core-skill".to_string(),
                vault: "shared-vault".to_string(),
                kind: AssetKind::Skill,
                version_constraint: Some(">=1.0.0".to_string()),
            }],
        };
        store.save(Scope::Workspace, &config).unwrap();
        let loaded = store.load(Scope::Workspace).unwrap();
        assert_eq!(loaded, config);
    }

    #[test]
    fn global_and_workspace_are_independent() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());

        let global_config = TeamConfig {
            name: "global-team".to_string(),
            ..TeamConfig::default()
        };
        store.save(Scope::Global, &global_config).unwrap();

        let ws_config = store.load(Scope::Workspace).unwrap();
        assert_eq!(ws_config.name, ""); // default
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());
        let config = TeamConfig {
            name: "test".to_string(),
            ..TeamConfig::default()
        };
        store.save(Scope::Workspace, &config).unwrap();
        assert!(dir.path().join(".agk").join("team.toml").exists());
    }

    #[test]
    fn exists_returns_false_when_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());
        // Only test Workspace scope — Global resolves to real home directory
        // which may have an existing team.toml from previous runs.
        assert!(!store.exists(Scope::Workspace));
    }

    #[test]
    fn exists_returns_true_after_save() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());
        let config = TeamConfig {
            name: "test".to_string(),
            ..TeamConfig::default()
        };
        store.save(Scope::Workspace, &config).unwrap();
        assert!(store.exists(Scope::Workspace));
    }
}