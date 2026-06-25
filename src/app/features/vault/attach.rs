use crate::app::outcome::CoreEventSink;
use crate::app::ports::{ClawHubPort, ConfigStorePort};
use crate::domain::config::{LocalVaultSource, VaultConfig};
use crate::domain::scope::Scope;
use anyhow::{Context, Result};
use std::path::Path;

/// Attach a vault definition to the global config.
///
/// Phase 3: Uses [`ConfigStorePort`] only — no direct filesystem access.
/// For ClawHub vaults, verifies the CLI is available and attempts
/// Homebrew installation when missing (via [`ClawHubPort`]).
pub fn run(
    vault_id: String,
    vault_config: VaultConfig,
    store: &dyn ConfigStorePort,
    clawhub: &dyn ClawHubPort,
    sink: &mut dyn CoreEventSink,
) -> Result<()> {
    // ClawHub auto-install: if the CLI is missing and Homebrew is present,
    // install automatically before attaching.
    if matches!(vault_config, VaultConfig::Clawhub(_)) && !clawhub.is_cli_available() {
        if clawhub.is_homebrew_available() {
            sink.on_event(crate::app::event::CoreEvent::Info(
                "ClawHub CLI not found — installing via Homebrew...".to_string(),
            ));
            if let Err(e) = clawhub.install_cli() {
                anyhow::bail!(
                    "ClawHub CLI is required but installation failed: {}. \
                     Install manually from https://clawhub.ai",
                    e
                );
            }
        } else {
            anyhow::bail!(
                "ClawHub CLI not found and Homebrew is unavailable. \
                 Install manually from https://clawhub.ai"
            );
        }
    }

    let mut config = store.load(Scope::Global)?;
    if !config.vaults.contains(&vault_id) {
        config.vaults.push(vault_id.clone());
    }
    let section = config.vault_defs.entry(vault_id.clone()).or_default();
    section.vault = Some(vault_config);
    store.save(Scope::Global, &config)?;
    sink.on_event(crate::app::event::CoreEvent::VaultAttached(vault_id));
    Ok(())
}

/// Attach a local vault directory by path.
///
/// Owns the business rules for registering an on-disk vault: it reads
/// `<path>/.agk/vault.toml`, parses the manifest, canonicalizes the path,
/// and delegates to [`run`]. Keeping this here (rather than in the CLI
/// adapter) means the TUI and CLI share one code path and the CLI
/// `to_core_command` stays a pure `CoreCommand` translator.
///
/// Errors when the manifest is missing, malformed, or the path cannot be
/// canonicalized — so the stored path is always deterministic.
pub fn attach_local(
    path: &str,
    id: Option<String>,
    store: &dyn ConfigStorePort,
    clawhub: &dyn ClawHubPort,
    sink: &mut dyn CoreEventSink,
) -> Result<()> {
    let vault_path = Path::new(path);
    let vault_toml = vault_path.join(".agk").join("vault.toml");
    if !vault_toml.exists() {
        anyhow::bail!(
            "No .agk/vault.toml found at '{}'. Run 'agk vault init' in that directory first.",
            vault_path.display()
        );
    }

    let manifest_content = std::fs::read_to_string(&vault_toml)
        .with_context(|| format!("Failed to read {}", vault_toml.display()))?;
    let manifest: crate::domain::vault_manifest::VaultManifest = toml::from_str(&manifest_content)
        .with_context(|| format!("Failed to parse {}", vault_toml.display()))?;

    // Canonicalize so the stored path is deterministic; fail loudly rather
    // than silently storing a non-canonical path.
    let abs_path = std::fs::canonicalize(vault_path)
        .with_context(|| format!("Failed to canonicalize vault path '{}'", vault_path.display()))?;

    let vault_id = id.unwrap_or_else(|| manifest.name.clone());
    let config = VaultConfig::Local(LocalVaultSource {
        path: abs_path.to_string_lossy().to_string(),
    });
    run(vault_id, config, store, clawhub, sink)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::ConfigStorePort;
    use crate::domain::config::{ConfigFile, VaultConfig};
    use crate::domain::scope::Scope;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeStore {
        data: Mutex<HashMap<String, ConfigFile>>,
    }

    impl FakeStore {
        fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    impl ConfigStorePort for FakeStore {
        fn load(&self, scope: Scope) -> Result<ConfigFile> {
            Ok(self
                .data
                .lock()
                .unwrap()
                .get(&format!("{:?}", scope))
                .cloned()
                .unwrap_or_default())
        }
        fn save(&self, scope: Scope, config: &ConfigFile) -> Result<()> {
            self.data
                .lock()
                .unwrap()
                .insert(format!("{:?}", scope), config.clone());
            Ok(())
        }
    }

    struct NullSink;
    impl crate::app::outcome::CoreEventSink for NullSink {
        fn on_event(&mut self, _event: crate::app::event::CoreEvent) {}
        fn on_error(&mut self, _error: String) {}
    }

    #[test]
    fn attach_vault_adds_to_config() {
        let store = FakeStore::new();
        let mut sink = NullSink;
        let clawhub = crate::app::test_support::FakeClawHub::new();
        let vault_config = VaultConfig::Local(crate::domain::config::LocalVaultSource {
            path: "/tmp/vault".into(),
        });
        run("my-vault".into(), vault_config, &store, &clawhub, &mut sink).unwrap();

        let config = store.load(Scope::Global).unwrap();
        assert!(config.vaults.contains(&"my-vault".to_string()));
        assert!(config.vault_defs.contains_key("my-vault"));
    }

    #[test]
    fn attach_local_reads_manifest_and_canonicalizes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".agk")).unwrap();
        std::fs::write(
            dir.path().join(".agk").join("vault.toml"),
            "name = \"manifest-vault\"\n",
        )
        .unwrap();

        let store = FakeStore::new();
        let mut sink = NullSink;
        let clawhub = crate::app::test_support::FakeClawHub::new();

        // No --id override: the vault id defaults to the manifest name.
        attach_local(
            dir.path().to_str().unwrap(),
            None,
            &store,
            &clawhub,
            &mut sink,
        )
        .unwrap();

        let config = store.load(Scope::Global).unwrap();
        assert!(config.vaults.contains(&"manifest-vault".to_string()));
        let section = config.vault_defs.get("manifest-vault").unwrap();
        let canonical = std::fs::canonicalize(dir.path()).unwrap();
        match section.vault.as_ref().unwrap() {
            VaultConfig::Local(src) => {
                assert_eq!(src.path, canonical.to_string_lossy());
            }
            other => panic!("expected local vault, got {:?}", other),
        }
    }

    #[test]
    fn attach_local_id_override_wins() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".agk")).unwrap();
        std::fs::write(
            dir.path().join(".agk").join("vault.toml"),
            "name = \"manifest-vault\"\n",
        )
        .unwrap();

        let store = FakeStore::new();
        let mut sink = NullSink;
        let clawhub = crate::app::test_support::FakeClawHub::new();

        attach_local(
            dir.path().to_str().unwrap(),
            Some("custom-id".into()),
            &store,
            &clawhub,
            &mut sink,
        )
        .unwrap();

        let config = store.load(Scope::Global).unwrap();
        assert!(config.vaults.contains(&"custom-id".to_string()));
        assert!(!config.vaults.contains(&"manifest-vault".to_string()));
    }

    #[test]
    fn attach_local_missing_manifest_errors() {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeStore::new();
        let mut sink = NullSink;
        let clawhub = crate::app::test_support::FakeClawHub::new();

        let result = attach_local(
            dir.path().to_str().unwrap(),
            None,
            &store,
            &clawhub,
            &mut sink,
        );

        assert!(result.is_err(), "missing .agk/vault.toml must error");
        // Nothing should have been written to the config.
        assert!(store.load(Scope::Global).unwrap().vaults.is_empty());
    }
}
