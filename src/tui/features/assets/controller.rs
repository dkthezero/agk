use crate::app::command::CoreCommand;
use crate::tui::app::AppState;
use crate::tui::event::{AppEvent, EventContext};
use anyhow::Result;

pub fn handle_space_asset(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let pkg_opt = {
        let filtered = state.filtered_packages();
        filtered.get(state.selected_index).copied().cloned()
    };
    if let Some(pkg) = pkg_opt {
        let is_installed = state.is_installed(&pkg.vault_id, &pkg.identity.name, &pkg.kind);
        let identity = pkg.identity.name.clone();
        let scope = state.active_scope;

        let cmd = if pkg.is_remote || !is_installed {
            CoreCommand::InstallAsset {
                identity,
                scope,
                provider_filter: None,
                include_evals: false,
                dry_run: false,
            }
        } else {
            CoreCommand::RemoveAsset {
                identity,
                scope,
                provider_filter: None,
            }
        };
        let _ = ctx.tx.send(AppEvent::ExecuteCommand(cmd));
    }
    Ok(())
}

pub fn handle_enter_update(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let active_kind = state
        .tab_kinds
        .get(state.active_tab)
        .cloned()
        .unwrap_or(crate::app::tab_kind::TabKind::Asset);
    if active_kind != crate::app::tab_kind::TabKind::Asset {
        state.status_line = "Update only applies to Skills/Instructions tabs".to_string();
    } else if !state.active_scope_has_provider() {
        let providers_idx = state
            .tab_names
            .iter()
            .position(|n| n == "Providers")
            .unwrap_or(3);
        crate::tui::features::common::actions::apply_space_no_provider(state, providers_idx);
    } else {
        let pkg_clone = {
            let filtered = state.filtered_packages();
            filtered.get(state.selected_index).map(|p| (*p).clone())
        };
        if let Some(pkg) = pkg_clone {
            let is_installed = state.is_installed(&pkg.vault_id, &pkg.identity.name, &pkg.kind);
            if !is_installed {
                state.status_line =
                    "Item not installed \u{2014} use Space to install first".to_string();
            } else {
                let _ = ctx
                    .tx
                    .send(AppEvent::ExecuteCommand(CoreCommand::UpdateAsset {
                        identity: pkg.identity.name.clone(),
                        scope: state.active_scope,
                        provider_filter: None,
                    }));
            }
        }
    }
    Ok(())
}

pub fn handle_f5_update_all(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let _ = ctx
        .tx
        .send(AppEvent::ExecuteCommand(CoreCommand::SyncAssets {
            scope: state.active_scope,
            dry_run: false,
        }));
    state.checked_items.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::{ConfigStorePort, FileOpenerPort, ProviderPort};
    use crate::domain::config::ConfigFile;
    use crate::domain::identity::AssetIdentity;
    use crate::domain::scope::Scope;

    use std::collections::HashMap;
    use std::sync::Arc;

    struct StubFileOpener;
    impl FileOpenerPort for StubFileOpener {
        fn open_file_manager(&self, _: &std::path::Path) -> anyhow::Result<()> {
            Ok(())
        }
        fn open_terminal(&self, _: &std::path::Path) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn empty_state(tab_count: usize) -> AppState {
        AppState::new(
            (0..tab_count).map(|i| format!("Tab{}", i)).collect(),
            vec![true; tab_count],
            HashMap::new(),
        )
    }

    struct FakeStore {
        config: std::sync::Mutex<ConfigFile>,
    }
    impl FakeStore {
        fn new(config: ConfigFile) -> Self {
            Self {
                config: std::sync::Mutex::new(config),
            }
        }
    }
    impl ConfigStorePort for FakeStore {
        fn load(&self, _scope: Scope) -> anyhow::Result<ConfigFile> {
            Ok(self.config.lock().unwrap().clone())
        }
        fn save(&self, _scope: Scope, config: &ConfigFile) -> anyhow::Result<()> {
            *self.config.lock().unwrap() = config.clone();
            Ok(())
        }
    }

    struct FakeProvider {
        id: String,
    }
    impl ProviderPort for FakeProvider {
        fn id(&self) -> &str {
            &self.id
        }
        fn name(&self) -> &str {
            &self.id
        }
        fn install(
            &self,
            _pkg: &crate::domain::asset::ScannedPackage,
            _scope: Scope,
            _config: Option<&crate::domain::config::ConfigFile>,
            _include_evals: bool,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _identity: &AssetIdentity,
            _kind: &crate::domain::asset::AssetKind,
            _scope: Scope,
            _config: Option<&crate::domain::config::ConfigFile>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_handle_space_emits_install_command() {
        let mut state = empty_state(1);
        let pkg = crate::domain::asset::ScannedPackage {
            identity: crate::domain::identity::AssetIdentity::new("my-skill", None, "hash"),
            path: std::path::PathBuf::from("a"),
            vault_id: "v".into(),
            kind: crate::domain::asset::AssetKind::Skill,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        };
        state.packages.insert(0, vec![pkg.clone()]);
        state.tab_kinds = vec![crate::app::tab_kind::TabKind::Asset];
        state.active_tab = 0;
        state.selected_index = 0;

        let mut config = ConfigFile::default();
        config.providers.push("fake".into());
        state.configs.insert(Scope::Workspace, config.clone());

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut registry = crate::app::registry::Registry::new();
        registry.register_provider(Box::new(FakeProvider { id: "fake".into() }));
        let registry = Arc::new(registry);

        let store = Arc::new(FakeStore::new(config));
        let ctx = EventContext {
            tx,
            workspace_root: std::path::PathBuf::from("."),
            file_opener: Arc::new(StubFileOpener),
            core: Arc::new(crate::app::core::test_core_with(store, registry)),
        };

        handle_space_asset(&mut state, &ctx).unwrap();

        let event = rx.try_recv().expect("Expected ExecuteCommand event");
        assert!(
            matches!(
                event,
                AppEvent::ExecuteCommand(CoreCommand::InstallAsset { ref identity, .. })
                if identity == "my-skill"
            ),
            "Expected InstallAsset command, got {:?}",
            event
        );
        assert!(rx.try_recv().is_err(), "Expected exactly one event");
    }

    #[tokio::test]
    async fn test_handle_space_emits_remove_command_when_installed() {
        let mut state = empty_state(1);
        let pkg = crate::domain::asset::ScannedPackage {
            identity: crate::domain::identity::AssetIdentity::new("my-skill", None, "hash"),
            path: std::path::PathBuf::from("a"),
            vault_id: "v".into(),
            kind: crate::domain::asset::AssetKind::Skill,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        };
        state.packages.insert(0, vec![pkg.clone()]);
        state.tab_kinds = vec![crate::app::tab_kind::TabKind::Asset];
        state.active_tab = 0;
        state.selected_index = 0;

        let mut config = ConfigFile::default();
        config.providers.push("fake".into());
        config.vault_defs.insert(
            "v".into(),
            crate::domain::config::VaultSection {
                vault: None,
                skills: Some(crate::domain::config::AssetBucket {
                    items: vec!["[my-skill:--:hash]".into()],
                }),
                instructions: None,
            },
        );
        state.configs.insert(Scope::Workspace, config.clone());

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut registry = crate::app::registry::Registry::new();
        registry.register_provider(Box::new(FakeProvider { id: "fake".into() }));
        let registry = Arc::new(registry);

        let store = Arc::new(FakeStore::new(config));
        let ctx = EventContext {
            tx,
            workspace_root: std::path::PathBuf::from("."),
            file_opener: Arc::new(StubFileOpener),
            core: Arc::new(crate::app::core::test_core_with(store, registry)),
        };

        handle_space_asset(&mut state, &ctx).unwrap();

        let event = rx.try_recv().expect("Expected ExecuteCommand event");
        assert!(
            matches!(
                event,
                AppEvent::ExecuteCommand(CoreCommand::RemoveAsset { ref identity, .. })
                if identity == "my-skill"
            ),
            "Expected RemoveAsset command, got {:?}",
            event
        );
    }
}
