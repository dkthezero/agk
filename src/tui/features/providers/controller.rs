use crate::tui::app::{AppState, ListMode};
use crate::tui::event::{AppEvent, ControlFlow, EventContext};
use anyhow::Result;
use crossterm::event::KeyCode;

pub fn handle_select_provider_root(
    state: &mut AppState,
    ctx: &EventContext,
    code: &KeyCode,
) -> Result<ControlFlow> {
    match code {
        KeyCode::Up => {
            if let ListMode::SelectProviderRoot { selected, .. } = &mut state.list_mode {
                *selected = selected.saturating_sub(1);
            }
            Ok(ControlFlow::Continue)
        }
        KeyCode::Down => {
            if let ListMode::SelectProviderRoot {
                selected, options, ..
            } = &mut state.list_mode
            {
                if *selected + 1 < options.len() {
                    *selected += 1;
                }
            }
            Ok(ControlFlow::Continue)
        }
        KeyCode::Enter => {
            if let ListMode::SelectProviderRoot {
                provider_id,
                options,
                selected,
            } = &state.list_mode
            {
                let chosen = options[*selected].0.clone();
                let mut config = state.active_config().clone();
                config.provider_roots.insert(provider_id.clone(), chosen);
                let scope = state.active_scope;
                match ctx.store.save(scope, &config) {
                    Ok(()) => {
                        state.configs.insert(scope, config);
                        state.list_mode = ListMode::Normal;
                        return toggle_provider(state, ctx);
                    }
                    Err(e) => {
                        state.status_line = format!("Failed to save config: {}", e);
                        return Ok(ControlFlow::Continue);
                    }
                }
            }
            Ok(ControlFlow::Continue)
        }
        KeyCode::Esc => {
            state.list_mode = ListMode::Normal;
            state.status_line = "Provider enable cancelled".to_string();
            Ok(ControlFlow::Continue)
        }
        _ => Ok(ControlFlow::Continue),
    }
}

pub fn handle_deactivate_last_provider_confirm(
    state: &mut AppState,
    ctx: &EventContext,
) -> Result<ControlFlow> {
    let provider_id = std::mem::take(&mut state.pending_deactivate_provider_id);
    state.list_mode = ListMode::Normal;

    let scope = state.active_scope;
    let store = ctx.store.clone();
    let tx = ctx.tx.clone();
    let registry = ctx.registry.clone();

    let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    tokio::task::spawn_blocking(move || {
        let _ = tx.send(AppEvent::TaskStarted {
            id,
            name: format!("Deactivating '{}'", provider_id),
        });

        let mut config = store.load(scope).unwrap_or_default();
        if !config.providers.contains(&provider_id) {
            let _ = tx.send(AppEvent::TaskFailed {
                id,
                error: "Provider already deactivated".into(),
            });
            return;
        }

        if let Ok(provider) = registry.get_provider(&provider_id) {
            config.providers.retain(|p| p != &provider_id);
            config.provider_roots.remove(&provider_id);

            for section in config.vault_defs.values() {
                if let Some(ref skills) = section.skills {
                    for item in &skills.items {
                        if let Some(identity) = crate::domain::config::parse_identity(item) {
                            let _ = provider.remove(
                                &identity,
                                &crate::domain::asset::AssetKind::Skill,
                                scope,
                                Some(&config),
                            );
                        }
                    }
                }
                if let Some(ref instructions) = section.instructions {
                    for item in &instructions.items {
                        if let Some(identity) = crate::domain::config::parse_identity(item) {
                            let _ = provider.remove(
                                &identity,
                                &crate::domain::asset::AssetKind::Instruction,
                                scope,
                                Some(&config),
                            );
                        }
                    }
                }
            }

            for section in config.vault_defs.values_mut() {
                if let Some(ref mut b) = section.skills {
                    b.items.clear();
                }
                if let Some(ref mut b) = section.instructions {
                    b.items.clear();
                }
            }
            crate::app::features::common::prune_empty_vault_defs(&mut config);

            if config == crate::domain::config::ConfigFile::default() {
                if let Err(e) = store.delete_file(scope) {
                    let _ = tx.send(AppEvent::TaskFailed {
                        id,
                        error: format!("Failed to delete empty config file: {}", e),
                    });
                    return;
                }
            } else {
                let _ = store.save(scope, &config);
            }
        }

        let _ = tx.send(AppEvent::TriggerReload);
        let _ = tx.send(AppEvent::TaskCompleted {
            id,
            message: format!("Deactivated '{}'", provider_id),
        });
    });

    Ok(ControlFlow::Continue)
}

pub fn handle_deactivate_last_provider_cancel(state: &mut AppState) -> Result<ControlFlow> {
    state.list_mode = ListMode::Normal;
    state.status_line = "Cancelled provider deactivation".to_string();
    state.pending_deactivate_provider_id.clear();
    Ok(ControlFlow::Continue)
}

pub fn handle_space_provider(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    if let Some(entry) = state.provider_entries.get(state.selected_index) {
        let provider = ctx.registry.providers.iter().find(|p| p.id() == entry.id);
        if let Some(p) = provider {
            if !entry.active && state.active_scope == crate::domain::scope::Scope::Workspace {
                let roots = p.available_config_roots();
                let already_selected = state.active_config().provider_roots.contains_key(&entry.id);
                if roots.len() > 1 && !already_selected {
                    state.list_mode = ListMode::SelectProviderRoot {
                        provider_id: entry.id.clone(),
                        options: roots,
                        selected: 0,
                    };
                    return Ok(());
                }
            }
        }
    }
    toggle_provider(state, ctx).map(|_| ())
}

pub fn toggle_provider(state: &mut AppState, ctx: &EventContext) -> Result<ControlFlow> {
    if let Some(p) = state.provider_entries.get(state.selected_index) {
        let provider_id = p.id.clone();
        let scope = state.active_scope;
        let store = ctx.store.clone();
        let tx = ctx.tx.clone();
        let registry = ctx.registry.clone();

        let mut installed_pkgs = Vec::new();
        for tab_pkgs in state.packages.values() {
            for pkg in tab_pkgs {
                if state.is_installed(&pkg.vault_id, &pkg.identity.name, &pkg.kind) {
                    installed_pkgs.push(pkg.clone());
                }
            }
        }

        let config = store.load(scope).unwrap_or_default();
        let is_last_provider =
            config.providers.len() == 1 && config.providers.contains(&provider_id);
        let has_installed_assets = config.vault_defs.values().any(|section| {
            section
                .skills
                .as_ref()
                .map(|b| !b.items.is_empty())
                .unwrap_or(false)
                || section
                    .instructions
                    .as_ref()
                    .map(|b| !b.items.is_empty())
                    .unwrap_or(false)
        });

        if is_last_provider && has_installed_assets {
            state.list_mode = ListMode::ConfirmDeactivateLastProvider;
            state.pending_deactivate_provider_id = provider_id;
            state.status_line.clear();
            return Ok(ControlFlow::Continue);
        }

        let id = crate::tui::app::NEXT_TASK_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        tokio::task::spawn_blocking(move || {
            let mut config = store.load(scope).unwrap_or_default();
            if config.providers.contains(&provider_id) {
                let _ = tx.send(AppEvent::TaskStarted {
                    id,
                    name: "Deactivating Provider".into(),
                });
                let total = installed_pkgs.len();
                if let Ok(provider) = registry.get_provider(&provider_id) {
                    config.providers.retain(|p| p != &provider_id);
                    config.provider_roots.remove(&provider_id);

                    for (i, pkg) in installed_pkgs.iter().enumerate() {
                        let _ = provider.remove(&pkg.identity, &pkg.kind, scope, Some(&config));
                        let percent = (((i + 1) as f32 / total.max(1) as f32) * 100.0) as u8;
                        let _ = tx.send(AppEvent::TaskProgress { id, percent });
                    }

                    if config.providers.is_empty() {
                        for section in config.vault_defs.values_mut() {
                            if let Some(ref mut b) = section.skills {
                                b.items.clear();
                            }
                            if let Some(ref mut b) = section.instructions {
                                b.items.clear();
                            }
                        }
                        crate::app::features::common::prune_empty_vault_defs(&mut config);

                        if config == crate::domain::config::ConfigFile::default() {
                            if let Err(e) = store.delete_file(scope) {
                                let _ = tx.send(AppEvent::TaskFailed {
                                    id,
                                    error: format!("Failed to delete empty config file: {}", e),
                                });
                                return;
                            }
                        } else {
                            let _ = store.save(scope, &config);
                        }
                    } else {
                        let _ = store.save(scope, &config);
                    }
                }
                let _ = tx.send(AppEvent::TriggerReload);
                let _ = tx.send(AppEvent::TaskCompleted {
                    id,
                    message: format!("Deactivated '{}'", provider_id),
                });
            } else {
                let _ = tx.send(AppEvent::TaskStarted {
                    id,
                    name: "Activating Provider".into(),
                });
                let total = installed_pkgs.len();
                if let Ok(provider) = registry.get_provider(&provider_id) {
                    config.providers.push(provider_id.clone());
                    let _ = store.save(scope, &config);

                    for (i, pkg) in installed_pkgs.iter().enumerate() {
                        let _ = crate::app::features::asset::install::install_asset(
                            scope,
                            pkg,
                            store.as_ref(),
                            provider,
                        );
                        let percent = (((i + 1) as f32 / total.max(1) as f32) * 100.0) as u8;
                        let _ = tx.send(AppEvent::TaskProgress { id, percent });
                    }
                }
                let _ = tx.send(AppEvent::TriggerReload);
                let _ = tx.send(AppEvent::TaskCompleted {
                    id,
                    message: format!("Activated '{}'", provider_id),
                });
            }
        });
    }
    Ok(ControlFlow::Continue)
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

    #[test]
    fn toggle_provider_shows_confirm_popup_when_last_provider_has_assets() {
        let mut state = empty_state(5);
        state.tab_kinds = vec![
            crate::app::tab_kind::TabKind::Asset,    // Skills
            crate::app::tab_kind::TabKind::Mcp,      // MCP
            crate::app::tab_kind::TabKind::Asset,    // Instructions
            crate::app::tab_kind::TabKind::Provider, // Providers
            crate::app::tab_kind::TabKind::Vault,    // Vaults
        ];
        state.active_tab = 3; // Providers tab
        state.provider_entries = vec![crate::app::snapshot::ProviderEntry {
            id: "fake".to_string(),
            name: "Fake".to_string(),
            active: true,
        }];

        let mut config = ConfigFile {
            providers: vec!["fake".to_string()],
            ..ConfigFile::default()
        };
        config.vault_defs.insert(
            "workspace".to_string(),
            crate::domain::config::VaultSection {
                vault: None,
                skills: Some(crate::domain::config::AssetBucket {
                    items: vec!["[my-skill:--:0000000000]".to_string()],
                }),
                instructions: None,
            },
        );
        state.configs.insert(Scope::Workspace, config.clone());

        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
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

        let res = toggle_provider(&mut state, &ctx).unwrap();

        assert!(matches!(res, ControlFlow::Continue));
        assert_eq!(
            state.list_mode,
            ListMode::ConfirmDeactivateLastProvider,
            "Expected confirm popup when deactivating last provider with installed assets"
        );
        assert_eq!(state.pending_deactivate_provider_id, "fake");
    }
}
