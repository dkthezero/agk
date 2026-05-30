use crate::app::ports::ConfigStorePort;
use crate::domain::config::ConfigFile;
use crate::domain::scope::Scope;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Mutex;

/// In-memory [`ConfigStorePort`] backed by a `HashMap` keyed by scope.
///
/// Useful for tests that need to assert config was written / loaded without
/// touching the filesystem.
#[derive(Debug)]
pub struct FakeStore {
    data: Mutex<HashMap<String, ConfigFile>>,
}

impl Default for FakeStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }

    /// Pre-seed a scope with a config so tests can start from a known state.
    pub fn seed(&self, scope: Scope, config: ConfigFile) {
        self.data.lock().unwrap().insert(scope_key(scope), config);
    }
}

impl ConfigStorePort for FakeStore {
    fn load(&self, scope: Scope) -> Result<ConfigFile> {
        Ok(self
            .data
            .lock()
            .unwrap()
            .get(&scope_key(scope))
            .cloned()
            .unwrap_or_default())
    }

    fn save(&self, scope: Scope, config: &ConfigFile) -> Result<()> {
        self.data
            .lock()
            .unwrap()
            .insert(scope_key(scope), config.clone());
        Ok(())
    }
}

fn scope_key(scope: Scope) -> String {
    format!("{:?}", scope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_store_round_trip() {
        let store = FakeStore::new();
        let mut config = ConfigFile::default();
        config.vaults.push("my-vault".into());
        store.save(Scope::Global, &config).unwrap();

        let loaded = store.load(Scope::Global).unwrap();
        assert_eq!(loaded.vaults, vec!["my-vault"]);
    }

    #[test]
    fn fake_store_default_empty() {
        let store = FakeStore::new();
        let loaded = store.load(Scope::Workspace).unwrap();
        assert!(loaded.vaults.is_empty());
        assert!(loaded.providers.is_empty());
    }

    #[test]
    fn fake_store_seed() {
        let store = FakeStore::new();
        let mut config = ConfigFile::default();
        config.providers.push("opencode".into());
        store.seed(Scope::Global, config);

        let loaded = store.load(Scope::Global).unwrap();
        assert_eq!(loaded.providers, vec!["opencode"]);
    }
}
