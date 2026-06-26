use crate::app::command::CoreCommand;
use crate::tui::app::AppState;
use crate::tui::event::{AppEvent, ControlFlow, EventContext};
use crate::tui::list_mode::ListMode;
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
                let provider_id = provider_id.clone();
                let mut config = state.active_config().clone();
                config.provider_roots.insert(provider_id.clone(), chosen);
                let scope = state.active_scope;
                match ctx.core.store.save(scope, &config) {
                    Ok(()) => {
                        state.configs.insert(scope, config);
                        state.list_mode = ListMode::Normal;
                        let _ =
                            ctx.tx
                                .send(AppEvent::ExecuteCommand(CoreCommand::ActivateProvider {
                                    id: provider_id,
                                    scope,
                                }));
                        return Ok(ControlFlow::Continue);
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
    let _ = ctx
        .tx
        .send(AppEvent::ExecuteCommand(CoreCommand::DeactivateProvider {
            id: provider_id,
            scope: state.active_scope,
        }));
    Ok(ControlFlow::Continue)
}

pub fn handle_deactivate_last_provider_cancel(state: &mut AppState) -> Result<ControlFlow> {
    state.list_mode = ListMode::Normal;
    state.status_line = "Cancelled provider deactivation".to_string();
    state.pending_deactivate_provider_id.clear();
    Ok(ControlFlow::Continue)
}

pub fn handle_space_provider(state: &mut AppState, ctx: &EventContext) -> Result<()> {
    if state.is_vault_workspace {
        state.status_line =
            "Providers are not used in vault source mode — skills install to skills/ directly"
                .to_string();
        return Ok(());
    }
    if let Some(entry) = state.provider_entries.get(state.selected_index) {
        let provider = ctx
            .core
            .registry
            .providers
            .iter()
            .find(|p| p.id() == entry.id);
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
        let config = state.active_config().clone();
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

        let cmd = if config.providers.contains(&provider_id) {
            CoreCommand::DeactivateProvider {
                id: provider_id,
                scope,
            }
        } else {
            CoreCommand::ActivateProvider {
                id: provider_id,
                scope,
            }
        };
        let _ = ctx.tx.send(AppEvent::ExecuteCommand(cmd));
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
            supports_mcp: false,
            supports_profiles: false,
            available_tools: vec![],
            available_permission_modes: vec![],
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
                    source: None,
                }),
                instructions: None,
                mcps: None,
                profiles: None,
            },
        );
        state.configs.insert(Scope::Workspace, config.clone());

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
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

        let res = toggle_provider(&mut state, &ctx).unwrap();

        assert!(matches!(res, ControlFlow::Continue));
        assert_eq!(
            state.list_mode,
            ListMode::ConfirmDeactivateLastProvider,
            "Expected confirm popup when deactivating last provider with installed assets"
        );
        assert_eq!(state.pending_deactivate_provider_id, "fake");
    }

    fn state_in_select_root_mode() -> AppState {
        let mut state = empty_state(5);
        state.active_scope = Scope::Workspace;
        state.configs.insert(
            Scope::Workspace,
            ConfigFile {
                providers: vec![],
                ..ConfigFile::default()
            },
        );
        state.list_mode = ListMode::SelectProviderRoot {
            provider_id: "opencode".to_string(),
            options: vec![
                (
                    ".opencode".to_string(),
                    "OpenCode native folder".to_string(),
                ),
                (".agents".to_string(), "Shared agents folder".to_string()),
            ],
            selected: 0,
        };
        state
    }

    fn event_context(
        store: Arc<FakeStore>,
    ) -> (EventContext, tokio::sync::mpsc::UnboundedReceiver<AppEvent>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut registry = crate::app::registry::Registry::new();
        registry.register_provider(Box::new(FakeProvider {
            id: "opencode".into(),
        }));
        let registry = Arc::new(registry);
        let ctx = EventContext {
            tx,
            workspace_root: std::path::PathBuf::from("."),
            file_opener: Arc::new(StubFileOpener),
            core: Arc::new(crate::app::core::test_core_with(store, registry)),
        };
        (ctx, rx)
    }

    #[test]
    fn select_provider_root_up_down_cycles_selection() {
        let mut state = state_in_select_root_mode();
        let store = Arc::new(FakeStore::new(state.active_config().clone()));
        let (ctx, _rx) = event_context(store);

        // Down moves selection 0 -> 1
        handle_select_provider_root(&mut state, &ctx, &KeyCode::Down).unwrap();
        if let ListMode::SelectProviderRoot { selected, .. } = &state.list_mode {
            assert_eq!(*selected, 1);
        } else {
            panic!("expected SelectProviderRoot mode");
        }

        // Down again is clamped (stays at last index)
        handle_select_provider_root(&mut state, &ctx, &KeyCode::Down).unwrap();
        if let ListMode::SelectProviderRoot { selected, .. } = &state.list_mode {
            assert_eq!(*selected, 1);
        } else {
            panic!("expected SelectProviderRoot mode");
        }

        // Up moves selection 1 -> 0
        handle_select_provider_root(&mut state, &ctx, &KeyCode::Up).unwrap();
        if let ListMode::SelectProviderRoot { selected, .. } = &state.list_mode {
            assert_eq!(*selected, 0);
        } else {
            panic!("expected SelectProviderRoot mode");
        }

        // Up at index 0 is clamped to 0 (saturating_sub)
        handle_select_provider_root(&mut state, &ctx, &KeyCode::Up).unwrap();
        if let ListMode::SelectProviderRoot { selected, .. } = &state.list_mode {
            assert_eq!(*selected, 0);
        } else {
            panic!("expected SelectProviderRoot mode");
        }
    }

    #[test]
    fn select_provider_root_enter_saves_choice_and_returns_to_normal() {
        let mut state = state_in_select_root_mode();
        // Move to second option (.agents)
        let store = Arc::new(FakeStore::new(state.active_config().clone()));
        let (ctx, mut rx) = event_context(store.clone());
        handle_select_provider_root(&mut state, &ctx, &KeyCode::Down).unwrap();

        // Enter confirms: config gets the chosen root, mode returns to Normal,
        // and an ActivateProvider command is sent.
        let res = handle_select_provider_root(&mut state, &ctx, &KeyCode::Enter).unwrap();
        assert!(matches!(res, ControlFlow::Continue));
        assert_eq!(state.list_mode, ListMode::Normal);
        let saved = store.config.lock().unwrap().clone();
        assert_eq!(
            saved.provider_roots.get("opencode"),
            Some(&".agents".to_string()),
            "Enter must persist the selected root into provider_roots"
        );
        // An ActivateProvider command should have been emitted on the channel.
        let emitted = rx.try_recv();
        assert!(
            matches!(
                emitted,
                Ok(AppEvent::ExecuteCommand(CoreCommand::ActivateProvider { ref id, .. }))
                    if id == "opencode"
            ),
            "Enter must emit ActivateProvider for the chosen provider, got {:?}",
            emitted
        );
    }

    #[test]
    fn select_provider_root_esc_cancels_without_persisting() {
        let mut state = state_in_select_root_mode();
        let store = Arc::new(FakeStore::new(state.active_config().clone()));
        let (ctx, _rx) = event_context(store.clone());

        let res = handle_select_provider_root(&mut state, &ctx, &KeyCode::Esc).unwrap();
        assert!(matches!(res, ControlFlow::Continue));
        assert_eq!(
            state.list_mode,
            ListMode::Normal,
            "Esc must cancel the modal and return to Normal"
        );
        assert!(
            state.status_line.contains("cancelled"),
            "Esc should surface a cancellation status line"
        );
        // Config must be unchanged (no provider_roots entry written)
        let saved = store.config.lock().unwrap().clone();
        assert!(
            saved.provider_roots.is_empty(),
            "Esc must not persist any selection"
        );
        assert!(
            !saved.providers.contains(&"opencode".to_string()),
            "Esc must not activate the provider"
        );
    }
}
