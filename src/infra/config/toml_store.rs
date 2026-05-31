use crate::app::ports::ConfigStorePort;
use crate::domain::config::ConfigFile;
use crate::domain::scope::Scope;
use anyhow::Result;
use std::path::PathBuf;

pub struct TomlConfigStore {
    global_path: PathBuf,
    workspace_path: PathBuf,
    lock: std::sync::Mutex<()>,
}

impl TomlConfigStore {
    pub fn new(global_path: PathBuf, workspace_path: PathBuf) -> Self {
        Self {
            global_path,
            workspace_path,
            lock: std::sync::Mutex::new(()),
        }
    }

    /// Construct with standard locations: ~/.config/agk/config.toml (global)
    /// and <workspace>/.agk/config.toml (workspace).
    pub fn standard(workspace_root: &std::path::Path) -> Self {
        let global = crate::domain::paths::global_config_root().join("config.toml");
        let workspace = workspace_root.join(".agk").join("config.toml");
        Self::new(global, workspace)
    }

    fn path_for(&self, scope: Scope) -> &PathBuf {
        match scope {
            Scope::Global => &self.global_path,
            Scope::Workspace => &self.workspace_path,
        }
    }
}

impl ConfigStorePort for TomlConfigStore {
    fn load(&self, scope: Scope) -> Result<ConfigFile> {
        let _guard = self
            .lock
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let path = self.path_for(scope);
        if !path.exists() {
            return Ok(ConfigFile::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: ConfigFile = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    fn save(&self, scope: Scope, config: &ConfigFile) -> Result<()> {
        let _guard = self
            .lock
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let path = self.path_for(scope);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(config)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn delete_file(&self, scope: Scope) -> Result<()> {
        let _guard = self
            .lock
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        let path = self.path_for(scope);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{AssetBucket, VaultSection};

    fn make_store(dir: &std::path::Path) -> TomlConfigStore {
        TomlConfigStore::new(
            dir.join("global").join("config.toml"),
            dir.join("workspace").join("config.toml"),
        )
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());
        let config = store.load(Scope::Global).unwrap();
        assert_eq!(config, ConfigFile::default());
    }

    #[test]
    fn round_trip_empty_config() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());
        let config = ConfigFile::default();
        store.save(Scope::Global, &config).unwrap();
        let loaded = store.load(Scope::Global).unwrap();
        assert_eq!(loaded, config);
    }

    #[test]
    fn round_trip_with_vault_and_skills() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());
        let mut config = ConfigFile {
            vaults: vec!["workspace".to_string()],
            providers: vec!["claude-code".to_string()],
            ..ConfigFile::default()
        };
        config.vault_defs.insert(
            "workspace".to_string(),
            VaultSection {
                vault: None,
                skills: Some(AssetBucket {
                    items: vec!["[my-skill:--:0000000000]".to_string()],
                }),
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        store.save(Scope::Workspace, &config).unwrap();
        let loaded = store.load(Scope::Workspace).unwrap();
        assert_eq!(loaded.vaults, vec!["workspace"]);
        assert_eq!(loaded.providers, vec!["claude-code"]);
        assert!(loaded.is_skill_installed("workspace", "my-skill"));
    }

    #[test]
    fn global_and_workspace_are_independent() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());
        let global = ConfigFile {
            providers: vec!["claude-code".to_string()],
            ..ConfigFile::default()
        };
        store.save(Scope::Global, &global).unwrap();
        let workspace = store.load(Scope::Workspace).unwrap();
        assert!(workspace.providers.is_empty());
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(dir.path());
        store.save(Scope::Global, &ConfigFile::default()).unwrap();
        assert!(dir.path().join("global").join("config.toml").exists());
    }
}
