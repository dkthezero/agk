use crate::app::command::CoreCommand;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::{
    ConfigStorePort, ContextStorePort, McpRegistryPort, ProfileRuntimePort, VaultSearchPort,
};
use crate::app::registry::Registry;
use std::collections::HashMap;
use std::sync::Arc;

/// Central façade through which all TUI and CLI commands enter the application
/// layer.  This is the only public API that interface adapters (`tui/` and `cli/`)
/// interact with directly.
///
/// Architecture rules enforced here:
/// - The façade receives **what** the user wants (`CoreCommand`).
/// - It delegates **how** to the private use-case implementations in `app/features/`.
/// - It emits facts back via [`CoreEventSink`] so adapters can render outcomes.
#[derive(Clone)]
pub struct AgkCore {
    pub(crate) store: Arc<dyn ConfigStorePort>,
    pub(crate) context_store: Arc<dyn ContextStorePort>,
    pub(crate) mcp_registry: Arc<dyn McpRegistryPort>,
    pub(crate) vault_search: Arc<dyn VaultSearchPort>,
    pub(crate) registry: Arc<Registry>,
    /// Provider runtime ports keyed by `provider_id` (e.g. "opencode").
    pub(crate) runtime_ports: HashMap<String, Arc<dyn ProfileRuntimePort>>,
}

impl AgkCore {
    pub fn new(
        store: Arc<dyn ConfigStorePort>,
        context_store: Arc<dyn ContextStorePort>,
        mcp_registry: Arc<dyn McpRegistryPort>,
        vault_search: Arc<dyn VaultSearchPort>,
        registry: Arc<Registry>,
        runtime_ports: HashMap<String, Arc<dyn ProfileRuntimePort>>,
    ) -> Self {
        Self {
            store,
            context_store,
            mcp_registry,
            vault_search,
            registry,
            runtime_ports,
        }
    }

    /// Execute a single [`CoreCommand`] to completion, streaming any
    /// intermediate events to the provided [`CoreEventSink`].
    ///
    /// The TUI passes a presenter that updates `TuiState`; the CLI passes a
    /// presenter that writes JSON / text to stdout.
    pub fn execute(&self, command: CoreCommand, sink: &mut dyn CoreEventSink) -> CoreResult {
        if let Some(r) = crate::app::features::profile::dispatch(&command, self, sink) {
            return r;
        }
        if let Some(r) = crate::app::features::vault::dispatch(&command, self, sink) {
            return r;
        }
        if let Some(r) = crate::app::features::context::dispatch(&command, self, sink) {
            return r;
        }
        if let Some(r) = crate::app::features::provider::dispatch(&command, self, sink) {
            return r;
        }
        if let Some(r) = crate::app::features::apply::dispatch(&command, self, sink) {
            return r;
        }
        if let Some(r) = crate::app::features::mcp::dispatch(&command, self, sink) {
            return r;
        }
        if let Some(r) = crate::app::features::asset::dispatch(&command, self, sink) {
            return r;
        }

        sink.on_error(format!("Command {:?} not yet implemented", command));
        Ok(CoreOutcome::Ok)
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

    struct FakeCtxStore;
    impl FakeCtxStore {
        fn new() -> Self {
            FakeCtxStore
        }
    }
    impl crate::app::ports::ContextStorePort for FakeCtxStore {
        fn load_contexts(&self) -> anyhow::Result<crate::domain::context::ContextFile> {
            Ok(Default::default())
        }
        fn save_contexts(
            &self,
            _contexts: &crate::domain::context::ContextFile,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn current_context(&self) -> anyhow::Result<crate::domain::context::ContextId> {
            Ok(crate::domain::context::ContextId::default())
        }
        fn switch_context(&self, _id: &crate::domain::context::ContextId) -> anyhow::Result<()> {
            Ok(())
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
            Arc::new(FakeCtxStore::new()),
            Arc::new(FakeMcp),
            Arc::new(FakeVaultSearch),
            Arc::new(crate::app::registry::Registry::new()),
            HashMap::new(),
        )
    }

    #[test]
    fn facade_executes_create_profile_stub() {
        let core = test_core();
        let mut sink = StubSink;
        let cmd = CoreCommand::CreateProfile {
            input: crate::app::features::profile::command::CreateProfileInput::new(
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
            input: crate::app::features::vault::command::AttachVaultInput {
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
