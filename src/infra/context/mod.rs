use crate::app::ports::ContextStorePort;
use crate::domain::context::{ContextFile, ContextId};
use anyhow::Result;
use std::path::PathBuf;

/// On-disk implementation of [`ContextStorePort`] using TOML files.
///
/// Stores:
/// - `<config_root>/contexts/contexts.toml` — the ContextFile
/// - `<config_root>/contexts/current-context` — a plain-text file with the active context name
pub struct TomlContextStore {
    contexts_path: PathBuf,
    current_context_path: PathBuf,
}

impl TomlContextStore {
    /// Build the store using the standard global config root.
    pub fn standard() -> Self {
        let root = crate::domain::paths::contexts_dir();
        Self::new(root)
    }

    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root: PathBuf = root.into();
        Self {
            contexts_path: root.join("contexts.toml"),
            current_context_path: root.join("current-context"),
        }
    }

    fn ensure_dir(&self) -> Result<()> {
        if let Some(parent) = self.contexts_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

impl ContextStorePort for TomlContextStore {
    fn load_contexts(&self) -> Result<ContextFile> {
        if !self.contexts_path.exists() {
            let mut file = ContextFile::default();
            file.ensure_default();
            return Ok(file);
        }
        let text = std::fs::read_to_string(&self.contexts_path)?;
        let mut file: ContextFile = toml::from_str(&text)?;
        file.ensure_default();
        Ok(file)
    }

    fn save_contexts(&self, contexts: &ContextFile) -> Result<()> {
        self.ensure_dir()?;
        let text = toml::to_string_pretty(contexts)?;
        std::fs::write(&self.contexts_path, text)?;
        Ok(())
    }

    fn current_context(&self) -> Result<ContextId> {
        let file = self.load_contexts()?;
        Ok(file.current_id())
    }

    fn switch_context(&self, id: &ContextId) -> Result<()> {
        let mut file = self.load_contexts()?;
        if !file.contexts.contains_key(id.as_str()) {
            anyhow::bail!("Context '{}' does not exist", id.as_str());
        }
        file.current_context = id.as_str().to_string();
        self.save_contexts(&file)?;
        // Also update the legacy current-context plain-text file for quick CLI reads.
        self.ensure_dir()?;
        std::fs::write(&self.current_context_path, id.as_str())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::context::{ContextConfig, ContextFile};

    #[test]
    fn missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlContextStore::new(dir.path());
        let file = store.load_contexts().unwrap();
        assert!(file.contexts.contains_key("default"));
        assert_eq!(file.current_context, "default");
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlContextStore::new(dir.path());

        let mut file = ContextFile::default();
        file.ensure_default();
        file.contexts.insert(
            "company-x".to_string(),
            ContextConfig {
                display_name: Some("Company X".to_string()),
                vaults: vec!["team".to_string()],
                ..ContextConfig::default()
            },
        );
        store.save_contexts(&file).unwrap();

        let loaded = store.load_contexts().unwrap();
        assert!(loaded.contexts.contains_key("company-x"));
        assert_eq!(
            loaded.contexts.get("company-x").unwrap().display_name,
            Some("Company X".to_string())
        );
    }

    #[test]
    fn switch_context_updates_current() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlContextStore::new(dir.path());

        let mut file = ContextFile::default();
        file.ensure_default();
        file.contexts.insert(
            "team-a".to_string(),
            ContextConfig {
                display_name: Some("Team A".to_string()),
                ..ContextConfig::default()
            },
        );
        store.save_contexts(&file).unwrap();

        store.switch_context(&ContextId::new("team-a")).unwrap();
        let current = store.current_context().unwrap();
        assert_eq!(current.as_str(), "team-a");
    }

    #[test]
    fn switch_missing_context_fails() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlContextStore::new(dir.path());
        let result = store.switch_context(&ContextId::new("missing"));
        assert!(result.is_err());
    }
}
