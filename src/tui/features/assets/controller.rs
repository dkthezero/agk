use crate::tui::app::AppState;
use crate::tui::event::{AppEvent, EventContext};
use anyhow::Result;

pub fn handle_space_asset(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let pkg_opt = {
        let filtered = state.filtered_packages();
        filtered.get(state.selected_index).copied().cloned()
    };
    if let Some(pkg) = pkg_opt {
        if pkg.is_remote {
            return handle_install_remote_clawhub(state, ctx, &pkg);
        }
        let is_installed = state.is_installed(&pkg.vault_id, &pkg.identity.name, &pkg.kind);
        let store = ctx.store.clone();
        let active_scope = state.active_scope;
        let tx = ctx.tx.clone();
        let registry = ctx.registry.clone();

        let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::task::spawn_blocking(move || {
            let action = if is_installed {
                "Uninstalling"
            } else {
                "Installing"
            };
            let _ = tx.send(AppEvent::TaskStarted {
                id,
                name: format!("{} '{}'", action, pkg.identity.name),
            });

            let config = store.load(active_scope).unwrap_or_default();
            let providers =
                crate::tui::features::common::actions::active_providers(&registry, &config);

            if providers.is_empty() {
                let _ = tx.send(AppEvent::TaskFailed {
                    id,
                    error: "No active providers to install to".into(),
                });
                return;
            }

            let mut success = true;
            for provider in providers {
                if is_installed {
                    if crate::app::features::asset::remove::remove_asset(
                        active_scope,
                        &pkg.identity,
                        &pkg.kind,
                        &pkg.vault_id,
                        store.as_ref(),
                        provider,
                    )
                    .is_err()
                    {
                        success = false;
                    }
                } else if crate::app::features::asset::install::install_asset(
                    active_scope,
                    &pkg,
                    store.as_ref(),
                    provider,
                )
                .is_err()
                {
                    success = false;
                }
            }
            let _ = tx.send(AppEvent::TaskProgress { id, percent: 100 });
            let _ = tx.send(AppEvent::TriggerReload);
            if success {
                let done = if is_installed {
                    "Uninstalled"
                } else {
                    "Installed"
                };
                let _ = tx.send(AppEvent::TaskCompleted {
                    id,
                    message: format!("{} '{}'", done, pkg.identity.name),
                });
            } else {
                let _ = tx.send(AppEvent::TaskFailed {
                    id,
                    error: format!(
                        "Failed to {} '{}'",
                        action.to_lowercase(),
                        pkg.identity.name
                    ),
                });
            }
        });
    }
    Ok(())
}

pub fn handle_install_remote_clawhub(
    state: &mut AppState,
    ctx: &EventContext,
    pkg: &crate::domain::asset::ScannedPackage,
) -> Result<()> {
    let slug = pkg.identity.name.clone();
    let store = ctx.store.clone();
    let tx = ctx.tx.clone();
    let registry = ctx.registry.clone();
    let active_scope = state.active_scope;

    let fetch_id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let install_id =
        crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    let _ = tx.send(AppEvent::TaskStarted {
        id: fetch_id,
        name: format!("Fetching '{}' from ClawHub", slug),
    });
    let _ = tx.send(AppEvent::TaskStarted {
        id: install_id,
        name: format!("Installing '{}' to {:?}", slug, active_scope),
    });

    tokio::task::spawn_blocking(move || {
        match crate::infra::vault::clawhub::cli_install(&slug) {
            Ok(()) => {
                let _ = tx.send(AppEvent::TaskProgress {
                    id: fetch_id,
                    percent: 100,
                });
                let _ = tx.send(AppEvent::TaskCompleted {
                    id: fetch_id,
                    message: format!("Fetched '{}' from ClawHub", slug),
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::TaskFailed {
                    id: fetch_id,
                    error: format!("Failed to fetch '{}': {}", slug, e),
                });
                let _ = tx.send(AppEvent::TaskFailed {
                    id: install_id,
                    error: "Cancelled — fetch failed".into(),
                });
                return;
            }
        }

        let cache_dir = crate::domain::paths::clawhub_cache_dir();
        let local = crate::infra::vault::local::LocalVaultAdapter::new("clawhub", cache_dir);
        let feature = crate::infra::feature::skill::SkillFeatureSet;
        use crate::app::ports::VaultPort;
        let cached_pkgs = match local.list_packages(&feature) {
            Ok(pkgs) => pkgs,
            Err(e) => {
                let _ = tx.send(AppEvent::TaskFailed {
                    id: install_id,
                    error: format!("Failed to scan cached package: {}", e),
                });
                return;
            }
        };

        let cached_pkg = cached_pkgs.iter().find(|p| p.identity.name == slug);
        if let Some(pkg) = cached_pkg {
            let config = store.load(active_scope).unwrap_or_default();
            let providers =
                crate::tui::features::common::actions::active_providers(&registry, &config);
            if providers.is_empty() {
                let _ = tx.send(AppEvent::TaskFailed {
                    id: install_id,
                    error: "No active providers to install to".into(),
                });
                return;
            }
            let mut success = true;
            for provider in providers {
                if crate::app::features::asset::install::install_asset(active_scope, pkg, store.as_ref(), provider)
                    .is_err()
                {
                    success = false;
                }
            }
            let _ = tx.send(AppEvent::TaskProgress {
                id: install_id,
                percent: 100,
            });
            let _ = tx.send(AppEvent::TriggerReload);
            if success {
                let _ = tx.send(AppEvent::TaskCompleted {
                    id: install_id,
                    message: format!("Installed '{}' to {:?}", slug, active_scope),
                });
            } else {
                let _ = tx.send(AppEvent::TaskFailed {
                    id: install_id,
                    error: format!("Failed to install '{}'", slug),
                });
            }
        } else {
            let _ = tx.send(AppEvent::TaskFailed {
                id: install_id,
                error: format!("Skill '{}' not found in ClawHub cache after fetch", slug),
            });
        }
    });
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
                let providers = if let Ok(config) = ctx.store.load(state.active_scope) {
                    crate::tui::features::common::actions::active_providers(&ctx.registry, &config)
                } else {
                    vec![]
                };

                if providers.is_empty() {
                    state.status_line = "No active providers to update to".to_string();
                } else {
                    let mut success = true;
                    for provider in providers {
                        if let Err(e) = crate::app::features::asset::update::update_asset(
                            state.active_scope,
                            &pkg,
                            ctx.store.as_ref(),
                            provider,
                        ) {
                            state.status_line =
                                format!("Update failed for {}: {}", provider.name(), e);
                            success = false;
                            break;
                        }
                    }
                    if success {
                        if let Ok(config) = ctx.store.load(state.active_scope) {
                            state.configs.insert(state.active_scope, config);
                        }
                        state.status_line = format!("Updated '{}'", pkg.identity.name);
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn handle_f5_update_all(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    let mut pkgs_to_update = Vec::new();
    for pkg_list in state.packages.values() {
        for pkg in pkg_list {
            if state.is_installed(&pkg.vault_id, &pkg.identity.name, &pkg.kind) {
                pkgs_to_update.push(pkg.clone());
            }
        }
    }

    if pkgs_to_update.is_empty() {
        state.status_line = "No installed items to update".into();
        return Ok(());
    }

    let tx = ctx.tx.clone();
    let store = ctx.store.clone();
    let registry = ctx.registry.clone();
    let scope = state.active_scope;
    let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    tokio::task::spawn_blocking(move || {
        let _ = tx.send(AppEvent::TaskStarted {
            id,
            name: format!("Updating {} items...", pkgs_to_update.len()),
        });

        let providers = if let Ok(config) = store.load(scope) {
            crate::tui::features::common::actions::active_providers(&registry, &config)
        } else {
            vec![]
        };

        if providers.is_empty() {
            let _ = tx.send(AppEvent::TaskFailed {
                id,
                error: "No active providers for update".into(),
            });
            return;
        }

        let mut success = 0;
        for pkg in pkgs_to_update {
            for provider in &providers {
                if crate::app::features::asset::update::update_asset(scope, &pkg, store.as_ref(), *provider).is_ok()
                {
                    success += 1;
                }
            }
        }
        let _ = tx.send(AppEvent::TaskProgress { id, percent: 100 });
        let _ = tx.send(AppEvent::TriggerReload);
        let _ = tx.send(AppEvent::TaskCompleted {
            id,
            message: format!("Updated {} items successfully", success),
        });
    });

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
    async fn test_handle_space_install() {
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
            store,
            registry,
            tx,
            workspace_root: std::path::PathBuf::from("."),
            file_opener: Arc::new(StubFileOpener),
        };

        handle_space_asset(&mut state, &ctx).unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut events = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }

        assert!(events
            .iter()
            .any(|e| matches!(e, AppEvent::TaskStarted { .. })));
        assert!(events.iter().any(|e| matches!(e, AppEvent::TriggerReload)));
        assert!(events
            .iter()
            .any(|e| matches!(e, AppEvent::TaskCompleted { .. })));
    }
}
