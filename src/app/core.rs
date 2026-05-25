use crate::app::command::CoreCommand;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::{ConfigStorePort, McpRegistryPort, VaultSearchPort};
use crate::app::registry::Registry;
use std::sync::Arc;

/// Central façade through which all TUI and CLI commands enter the application
/// layer.  This is the only public API that interface adapters (`tui/` and `cli/`)
/// interact with directly.
///
/// Architecture rules enforced here:
/// - The façade receives **what** the user wants (`CoreCommand`).
/// - It delegates **how** to the private use-case implementations in `app/usecases/`.
/// - It emits facts back via [`CoreEventSink`] so adapters can render outcomes.
#[derive(Clone)]
pub struct AgkCore {
    store: Arc<dyn ConfigStorePort>,
    mcp_registry: Arc<dyn McpRegistryPort>,
    vault_search: Arc<dyn VaultSearchPort>,
    registry: Arc<Registry>,
}

impl AgkCore {
    pub fn new(
        store: Arc<dyn ConfigStorePort>,
        mcp_registry: Arc<dyn McpRegistryPort>,
        vault_search: Arc<dyn VaultSearchPort>,
        registry: Arc<Registry>,
    ) -> Self {
        Self {
            store,
            mcp_registry,
            vault_search,
            registry,
        }
    }

    /// Execute a single [`CoreCommand`] to completion, streaming any
    /// intermediate events to the provided [`CoreEventSink`].
    ///
    /// The TUI passes a presenter that updates `TuiState`; the CLI passes a
    /// presenter that writes JSON / text to stdout.
    pub fn execute(&self, command: CoreCommand, sink: &mut dyn CoreEventSink) -> CoreResult {
        match &command {
            // ===============================================================
            // Profile commands
            // ===============================================================
            CoreCommand::CreateProfile { input } => {
                crate::app::usecases::create_profile::run(input, sink)
            }
            CoreCommand::StartProfile { id, scope, dry_run } => {
                crate::app::usecases::start_profile::run(id, *scope, *dry_run, sink)
            }

            // ===============================================================
            // Vault commands (wired)
            // ===============================================================
            CoreCommand::AttachVault { input } => crate::app::usecases::attach_vault::run(
                input.vault_id.clone(),
                input.config.clone(),
                self.store.as_ref(),
                sink,
            )
            .map(|_| CoreOutcome::Ok)
            .map_err(Into::into),

            // ===============================================================
            // Provider commands (stubbed — requires ProviderPort lookup)
            // ===============================================================
            CoreCommand::DeactivateProvider { id, scope } => match self.registry.get_provider(id) {
                Ok(provider) => crate::app::usecases::deactivate_provider::run(
                    id.clone(),
                    *scope,
                    self.store.as_ref(),
                    provider,
                    sink,
                ),
                Err(e) => {
                    sink.on_error(format!("Provider '{}' not found: {}", id, e));
                    Ok(CoreOutcome::Ok)
                }
            },

            // ===============================================================
            // MCP commands (wired)
            // ===============================================================
            CoreCommand::RegisterMcp { input } => crate::app::usecases::register_mcp::run(
                input.name.clone(),
                input.command.clone(),
                input.args.clone(),
                format!("{:?}", input.transport),
                input.description.clone(),
                input.test_after,
                sink,
            ),

            // ===============================================================
            // Asset / search commands
            // ===============================================================
            CoreCommand::SearchRemoteVault { vault_id, query } => {
                let _ = vault_id;
                let _ = query;
                // Phase 3.5: wire vault_search port
                sink.on_error("SearchRemoteVault not yet wired in AgkCore".into());
                Ok(CoreOutcome::Ok)
            }

            // TODO: Phase 4 – wire remaining commands.
            _ => {
                sink.on_error(format!("Command {:?} not yet implemented", command));
                Ok(CoreOutcome::Ok)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::ConfigFile;
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
        fn load(&self, scope: Scope) -> anyhow::Result<ConfigFile> {
            Ok(self
                .data
                .lock()
                .unwrap()
                .get(&format!("{:?}", scope))
                .cloned()
                .unwrap_or_default())
        }
        fn save(&self, scope: Scope, config: &ConfigFile) -> anyhow::Result<()> {
            self.data
                .lock()
                .unwrap()
                .insert(format!("{:?}", scope), config.clone());
            Ok(())
        }
    }

    struct FakeMcp;
    impl McpRegistryPort for FakeMcp {
        fn register(
            &self,
            _name: &str,
            _command: &str,
            _args: Option<&str>,
            _env: Option<&str>,
            _transport: &str,
            _description: Option<&str>,
        ) -> anyhow::Result<crate::domain::mcp::McpServer> {
            unimplemented!()
        }
        fn test_server(&self, _name: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn build_providers(
            &self,
            _workspace_root: &std::path::Path,
        ) -> Vec<Box<dyn crate::app::ports::McpProvider>> {
            vec![]
        }
        fn enable(&self, _name: &str, _provider_id: &str, _scope: Scope) -> anyhow::Result<()> {
            Ok(())
        }
        fn disable(&self, _name: &str, _provider_id: &str, _scope: Scope) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct FakeVaultSearch;
    #[async_trait::async_trait]
    impl VaultSearchPort for FakeVaultSearch {
        fn vault_id(&self) -> &str {
            "fake"
        }
        async fn search(
            &self,
            _query: &str,
        ) -> anyhow::Result<Vec<crate::domain::asset::ScannedPackage>> {
            Ok(vec![])
        }
    }

    struct StubSink;
    impl CoreEventSink for StubSink {
        fn on_event(&mut self, _event: crate::app::event::CoreEvent) {}
        fn on_error(&mut self, _error: String) {}
    }

    fn test_core() -> AgkCore {
        AgkCore::new(
            Arc::new(FakeStore::new()),
            Arc::new(FakeMcp),
            Arc::new(FakeVaultSearch),
            Arc::new(crate::app::registry::Registry::new()),
        )
    }

    #[test]
    fn facade_executes_create_profile_stub() {
        let core = test_core();
        let mut sink = StubSink;
        let cmd = CoreCommand::CreateProfile {
            input: crate::app::command::CreateProfileInput::new(
                "test-core",
                "opencode",
                crate::domain::scope::Scope::Workspace,
            ),
        };
        let result = core.execute(cmd, &mut sink);
        assert!(result.is_ok());
    }

    #[test]
    fn facade_routes_attach_vault() {
        let core = test_core();
        let mut sink = StubSink;
        let cmd = CoreCommand::AttachVault {
            input: crate::app::command::AttachVaultInput {
                vault_id: "test".into(),
                config: crate::domain::config::VaultConfig::Local(
                    crate::domain::config::LocalVaultSource {
                        path: "/tmp".into(),
                    },
                ),
                scope: Scope::Global,
            },
        };
        let result = core.execute(cmd, &mut sink);
        assert!(result.is_ok());
    }
}
