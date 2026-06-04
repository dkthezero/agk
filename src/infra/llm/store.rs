use crate::app::ports::llm_provider::LlmProviderStorePort;
use crate::domain::llm_provider::LlmProviderConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Persists `LlmProviderConfig`s in a sidecar TOML file under
/// `<agk_config_dir>/llm_providers.toml`. The store does not touch the main
/// `ConfigFile` schema — it is its own file so the two can evolve
/// independently and so the slim build (no `llm-*` features) does not have
/// to compile the serialization code.
pub struct FileLlmProviderStore<'a> {
    path: &'a Path,
}

#[derive(Serialize, Deserialize, Default)]
struct ProvidersFile {
    providers: Vec<LlmProviderConfig>,
}

impl<'a> FileLlmProviderStore<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self { path }
    }

    fn load_all(&self) -> Result<Vec<LlmProviderConfig>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let s = std::fs::read_to_string(self.path)
            .with_context(|| format!("reading {}", self.path.display()))?;
        if s.trim().is_empty() {
            return Ok(vec![]);
        }
        let file: ProvidersFile = toml::from_str(&s)
            .with_context(|| format!("parsing {}", self.path.display()))?;
        Ok(file.providers)
    }

    fn save_all(&self, cfgs: &[LlmProviderConfig]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("mkdir {}", parent.display()))?;
        }
        // The on-disk format wraps the list under a `providers` table because
        // the toml crate does not support top-level arrays.
        let file = ProvidersFile {
            providers: cfgs.to_vec(),
        };
        let s = toml::to_string_pretty(&file).context("serializing LlmProviderConfig list")?;
        std::fs::write(self.path, s)
            .with_context(|| format!("writing {}", self.path.display()))?;
        Ok(())
    }
}

impl<'a> LlmProviderStorePort for FileLlmProviderStore<'a> {
    fn list(&self) -> Result<Vec<LlmProviderConfig>> {
        self.load_all()
    }
    fn get(&self, id: &str) -> Result<Option<LlmProviderConfig>> {
        Ok(self.load_all()?.into_iter().find(|c| c.id == id))
    }
    fn upsert(&self, cfg: &LlmProviderConfig) -> Result<()> {
        let mut all = self.load_all()?;
        if let Some(existing) = all.iter_mut().find(|c| c.id == cfg.id) {
            *existing = cfg.clone();
        } else {
            all.push(cfg.clone());
        }
        self.save_all(&all)
    }
    fn remove(&self, id: &str) -> Result<()> {
        let mut all = self.load_all()?;
        all.retain(|c| c.id != id);
        self.save_all(&all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::LlmProviderStorePort;
    use crate::domain::llm_provider::{LlmProviderConfig, LlmProviderKind};
    use tempfile::tempdir;

    #[test]
    fn store_persists_across_instances() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("agk.toml");
        let s1 = FileLlmProviderStore::new(&path);
        s1.upsert(&LlmProviderConfig {
            id: "a".into(),
            kind: LlmProviderKind::Ollama,
            endpoint: "http://x".into(),
            api_key: None,
            default_model: None,
        })
        .unwrap();
        let s2 = FileLlmProviderStore::new(&path);
        let list = s2.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "a");
    }

    #[test]
    fn store_remove_drops_entry() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("agk.toml");
        let s = FileLlmProviderStore::new(&path);
        s.upsert(&LlmProviderConfig {
            id: "a".into(),
            kind: LlmProviderKind::Ollama,
            endpoint: "http://x".into(),
            api_key: None,
            default_model: None,
        })
        .unwrap();
        s.remove("a").unwrap();
        assert!(s.list().unwrap().is_empty());
    }
}
