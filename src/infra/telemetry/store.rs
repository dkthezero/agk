//! File-backed analytics store.
//!
//! Concrete `TelemetryStorePort` implementation. The actual file I/O lived in
//! `domain/telemetry.rs::AnalyticsConfig::{load,save}` until ADR-001 Commit 1.
//! Those inherent impls now live here so the domain type stays pure while
//! existing callers (`AnalyticsConfig::load(&path)`, `config.save(&path)`)
//! continue to compile unchanged.

use crate::app::ports::TelemetryStorePort;
use crate::domain::telemetry::AnalyticsConfig;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Default, Clone, Copy)]
pub struct FileTelemetryStore;

impl TelemetryStorePort for FileTelemetryStore {
    fn load(&self, path: &Path) -> Result<AnalyticsConfig> {
        load_from(path)
    }

    fn save(&self, path: &Path, config: &AnalyticsConfig) -> Result<()> {
        save_to(path, config)
    }
}

fn load_from(path: &Path) -> Result<AnalyticsConfig> {
    if !path.exists() {
        return Ok(AnalyticsConfig::default());
    }
    let content = std::fs::read_to_string(path)?;
    let config: AnalyticsConfig = toml::from_str(&content)?;
    Ok(config)
}

fn save_to(path: &Path, config: &AnalyticsConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

// Inherent impl moved out of `domain/telemetry.rs` so the domain layer has no
// file I/O. Existing call sites (`AnalyticsConfig::load(&path)` etc.) keep
// working — they just resolve to this impl block.
impl AnalyticsConfig {
    pub fn load(path: &Path) -> Result<Self> {
        load_from(path)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        save_to(path, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("analytics.toml");

        let mut config = AnalyticsConfig::default();
        config.settings.enabled = true;
        config.increment_invocation("web-browser", "claude-code");
        config.save(&path).unwrap();

        let loaded = AnalyticsConfig::load(&path).unwrap();
        assert!(loaded.settings.enabled);
        let skill = loaded.skills.get("web-browser").unwrap();
        assert_eq!(skill.total_invocations, 1);
        assert!(skill.providers().contains(&"claude-code".to_string()));
        assert_eq!(skill.provider_counts.get("claude-code"), Some(&1));
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist.toml");
        let loaded = AnalyticsConfig::load(&path).unwrap();
        assert!(!loaded.skills.iter().any(|_| true));
    }

    #[test]
    fn port_trait_load_save_matches_inherent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("analytics.toml");
        let store = FileTelemetryStore;
        let mut config = AnalyticsConfig::default();
        config.increment_invocation("skill-a", "claude-code");
        store.save(&path, &config).unwrap();
        let loaded = store.load(&path).unwrap();
        assert_eq!(
            loaded.skills.get("skill-a").map(|s| s.total_invocations),
            Some(1)
        );
    }
}
