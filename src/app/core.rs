use crate::app::command::CoreCommand;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::{
    ClawHubPort, ConfigStorePort, ContextStorePort, McpRegistryPort, ProcessRunnerPort,
    ProfileRuntimePort, TaskTrackerPort, TeamConfigStorePort, VaultSearchPort,
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
    pub store: Arc<dyn ConfigStorePort>,
    pub context_store: Arc<dyn ContextStorePort>,
    pub mcp_registry: Arc<dyn McpRegistryPort>,
    pub vault_search: Arc<dyn VaultSearchPort>,
    pub registry: Arc<Registry>,
    /// Provider runtime ports keyed by `provider_id` (e.g. "opencode").
    pub runtime_ports: HashMap<String, Arc<dyn ProfileRuntimePort>>,
    pub process_runner: Arc<dyn ProcessRunnerPort>,
    pub task_tracker: Arc<dyn TaskTrackerPort>,
    pub workspace_root: std::path::PathBuf,
    pub clawhub: Arc<dyn ClawHubPort>,
    pub team_config_store: Arc<dyn TeamConfigStorePort>,
}

impl AgkCore {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        store: Arc<dyn ConfigStorePort>,
        context_store: Arc<dyn ContextStorePort>,
        mcp_registry: Arc<dyn McpRegistryPort>,
        vault_search: Arc<dyn VaultSearchPort>,
        registry: Arc<Registry>,
        runtime_ports: HashMap<String, Arc<dyn ProfileRuntimePort>>,
        process_runner: Arc<dyn ProcessRunnerPort>,
        task_tracker: Arc<dyn TaskTrackerPort>,
        workspace_root: std::path::PathBuf,
        clawhub: Arc<dyn ClawHubPort>,
        team_config_store: Arc<dyn TeamConfigStorePort>,
    ) -> Self {
        Self {
            store,
            context_store,
            mcp_registry,
            vault_search,
            registry,
            runtime_ports,
            process_runner,
            task_tracker,
            workspace_root,
            clawhub,
            team_config_store,
        }
    }

    /// Execute a single [`CoreCommand`] to completion, streaming any
    /// intermediate events to the provided [`CoreEventSink`].
    ///
    /// The TUI passes a presenter that updates `TuiState`; the CLI passes a
    /// presenter that writes JSON / text to stdout.
    #[cfg_attr(
        feature = "observability",
        tracing::instrument(skip_all, fields(command = ?command))
    )]
    pub fn execute(&self, command: CoreCommand, sink: &mut dyn CoreEventSink) -> CoreResult {
        if let Some(r) = crate::app::features::profile::dispatch(&command, self, sink) {
            return r;
        }
        if let Some(r) = crate::app::features::vault::dispatch(&command, self, sink) {
            return r;
        }
        if let Some(r) = crate::app::features::team::dispatch(&command, self, sink) {
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
        if let Some(r) = crate::app::features::common::dispatch(&command, self, sink) {
            return r;
        }
        if let Some(r) = crate::app::features::telemetry::dispatch(&command, self, sink) {
            return r;
        }
        if let Some(r) = crate::app::features::debug::dispatch(&command, self, sink) {
            return r;
        }

        sink.on_error(format!("Command {:?} not yet implemented", command));
        Ok(CoreOutcome::Ok)
    }
}

#[cfg(test)]
pub use tests::test_core;
#[cfg(test)]
pub use tests::test_core_with;

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
        fn list(&self) -> anyhow::Result<Vec<crate::domain::mcp::McpServer>> {
            Ok(vec![])
        }
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

        fn unregister(&self, _name: &str) -> anyhow::Result<()> {
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

    struct RecordingSink {
        events: Vec<crate::app::event::CoreEvent>,
        errors: Vec<String>,
    }
    impl RecordingSink {
        fn new() -> Self {
            Self {
                events: Vec::new(),
                errors: Vec::new(),
            }
        }
    }
    impl CoreEventSink for RecordingSink {
        fn on_event(&mut self, event: crate::app::event::CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, error: String) {
            self.errors.push(error);
        }
    }

    struct FakeProcessRunner;
    impl ProcessRunnerPort for FakeProcessRunner {
        fn run(
            &self,
            _command: &str,
            _args: &[&str],
            _cwd: Option<&std::path::Path>,
            _env: Option<&[(String, String)]>,
        ) -> anyhow::Result<String> {
            Ok(String::new())
        }
    }

    struct FakeProvider;
    impl crate::app::ports::ProviderPort for FakeProvider {
        fn id(&self) -> &str {
            "opencode"
        }
        fn name(&self) -> &str {
            "OpenCode"
        }
        fn install(
            &self,
            _: &crate::domain::asset::ScannedPackage,
            _: crate::domain::scope::Scope,
            _: Option<&crate::domain::config::ConfigFile>,
            _: bool,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _: &crate::domain::identity::AssetIdentity,
            _: &crate::domain::asset::AssetKind,
            _: crate::domain::scope::Scope,
            _: Option<&crate::domain::config::ConfigFile>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn supports_profiles(&self) -> bool {
            true
        }
    }

    pub fn test_core() -> AgkCore {
        let mut registry = crate::app::registry::Registry::new();
        registry.register_provider(Box::new(FakeProvider));
        AgkCore::new(
            Arc::new(FakeStore::new()),
            Arc::new(FakeCtxStore::new()),
            Arc::new(FakeMcp),
            Arc::new(FakeVaultSearch),
            Arc::new(registry),
            HashMap::new(),
            Arc::new(FakeProcessRunner),
            Arc::new(crate::infra::task_tracker::InMemoryTaskTracker::new()),
            std::path::PathBuf::from("."),
            Arc::new(crate::app::test_support::FakeClawHub::new()),
            Arc::new(crate::app::test_support::FakeTeamConfigStore::new()),
        )
    }

    pub fn test_core_with(
        store: Arc<dyn crate::app::ports::ConfigStorePort>,
        registry: Arc<crate::app::registry::Registry>,
    ) -> AgkCore {
        AgkCore::new(
            store,
            Arc::new(FakeCtxStore::new()),
            Arc::new(FakeMcp),
            Arc::new(FakeVaultSearch),
            registry,
            HashMap::new(),
            Arc::new(FakeProcessRunner),
            Arc::new(crate::infra::task_tracker::InMemoryTaskTracker::new()),
            std::path::PathBuf::from("."),
            Arc::new(crate::app::test_support::FakeClawHub::new()),
            Arc::new(crate::app::test_support::FakeTeamConfigStore::new()),
        )
    }

    #[test]
    fn agk_core_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AgkCore>();
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

    /// Contract parity: the same [`CoreCommand`] must produce the same
    /// [`CoreEvent`] sequence regardless of which adapter (CLI or TUI) drives
    /// the core.  This is the central invariant of ADR-001.
    #[test]
    fn tui_cli_equivalence_profile_create() {
        let cli_core = test_core();
        let tui_core = test_core();

        let cmd = CoreCommand::CreateProfile {
            input: crate::app::features::profile::command::CreateProfileInput::new(
                "parity-test",
                "opencode",
                crate::domain::scope::Scope::Workspace,
            ),
        };

        let mut cli_sink = RecordingSink::new();
        let mut tui_sink = RecordingSink::new();

        let cli_result = cli_core.execute(cmd.clone(), &mut cli_sink);
        let tui_result = tui_core.execute(cmd, &mut tui_sink);

        assert_eq!(
            cli_result.is_ok(),
            tui_result.is_ok(),
            "CLI and TUI must agree on success/failure"
        );
        assert_eq!(
            cli_sink.events, tui_sink.events,
            "CLI and TUI must observe identical CoreEvent sequence"
        );
        assert_eq!(
            cli_sink.errors, tui_sink.errors,
            "CLI and TUI must observe identical error sequence"
        );
    }
}
