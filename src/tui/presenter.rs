use crate::app::event::CoreEvent;
use crate::app::outcome::CoreEventSink;
use crate::tui::event::AppEvent;
use tokio::sync::mpsc::UnboundedSender;

/// TUI presenter: bridges [`CoreEventSink`] into the async event loop.
///
/// Every `on_event` / `on_error` call sends an [`AppEvent::CoreEvent`] back
/// into the same channel consumed by [`crate::tui::runtime_loop::run_loop`].
/// This keeps the TUI single-threaded for state mutations while allowing
/// [`AgkCore`] to run in `spawn_blocking`.
pub struct TuiPresenter {
    tx: UnboundedSender<AppEvent>,
}

impl TuiPresenter {
    pub fn new(tx: UnboundedSender<AppEvent>) -> Self {
        Self { tx }
    }
}

impl CoreEventSink for TuiPresenter {
    fn on_event(&mut self, event: CoreEvent) {
        let _ = self.tx.send(AppEvent::CoreEvent(event));
    }

    fn on_error(&mut self, error: String) {
        let _ = self.tx.send(AppEvent::CoreEvent(CoreEvent::Error(error)));
    }
}

#[cfg(test)]
mod tests {
    //! CLI ↔ TUI parity contract test.
    //!
    //! Headline ADR-001 invariant: the same `CoreCommand` executed through
    //! `AgkCore` emits identical `CoreEvent` sequences regardless of whether
    //! the adapter is the CLI (`Recorder` sink directly) or the TUI
    //! (`TuiPresenter` → `tokio::mpsc` channel → drained back).
    //!
    //! Lives in `tui/presenter.rs` because integration tests under `tests/`
    //! cannot reach internals — the `agk` crate is a `[[bin]]` only, no
    //! `[lib]` target.

    use super::*;
    use crate::app::command::CoreCommand;
    use crate::app::core::AgkCore;
    use crate::app::event::CoreEvent;
    use crate::app::ports::{
        ConfigStorePort, ContextStorePort, McpProvider, McpRegistryPort, ProcessRunnerPort,
        VaultSearchPort,
    };
    use crate::app::registry::Registry;
    use crate::domain::context::{ContextConfig, ContextFile, ContextId};
    use crate::domain::mcp::McpServer;
    use crate::domain::scope::Scope;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Single normalized sequence of calls a sink received. Equating two
    /// `SinkCall` sequences proves the underlying `AgkCore` execution path
    /// is identical from both adapters' point of view.
    #[derive(Debug, Clone, PartialEq)]
    #[allow(clippy::large_enum_variant)] // test fixture; size optimisation unnecessary
    enum SinkCall {
        Event(CoreEvent),
        Error(String),
    }

    #[derive(Default)]
    struct Recorder {
        calls: Vec<SinkCall>,
    }
    impl CoreEventSink for Recorder {
        fn on_event(&mut self, event: CoreEvent) {
            self.calls.push(SinkCall::Event(event));
        }
        fn on_error(&mut self, error: String) {
            self.calls.push(SinkCall::Error(error));
        }
    }

    // ─── Minimum stubs to construct AgkCore ────────────────────────────────

    struct StubConfigStore;
    impl ConfigStorePort for StubConfigStore {
        fn load(&self, _scope: Scope) -> anyhow::Result<crate::domain::config::ConfigFile> {
            Ok(Default::default())
        }
        fn save(
            &self,
            _scope: Scope,
            _config: &crate::domain::config::ConfigFile,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct FixedContextStore {
        file: ContextFile,
    }
    impl ContextStorePort for FixedContextStore {
        fn load_contexts(&self) -> anyhow::Result<ContextFile> {
            Ok(self.file.clone())
        }
        fn save_contexts(&self, _: &ContextFile) -> anyhow::Result<()> {
            Ok(())
        }
        fn current_context(&self) -> anyhow::Result<ContextId> {
            Ok(ContextId::new(self.file.current_context.clone()))
        }
        fn switch_context(&self, _: &ContextId) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct StubMcp;
    impl McpRegistryPort for StubMcp {
        fn register(
            &self,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
            _: &str,
            _: Option<&str>,
        ) -> anyhow::Result<McpServer> {
            anyhow::bail!("stub")
        }
        fn list(&self) -> anyhow::Result<Vec<McpServer>> {
            Ok(vec![])
        }
        fn test_server(&self, _: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn build_providers(&self, _: &std::path::Path) -> Vec<Box<dyn McpProvider>> {
            vec![]
        }
        fn enable(&self, _: &str, _: &str, _: Scope) -> anyhow::Result<()> {
            Ok(())
        }
        fn disable(&self, _: &str, _: &str, _: Scope) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct StubVaultSearch;
    #[async_trait::async_trait]
    impl VaultSearchPort for StubVaultSearch {
        fn vault_id(&self) -> &str {
            "stub"
        }
        async fn search(
            &self,
            _: &str,
        ) -> anyhow::Result<Vec<crate::domain::asset::ScannedPackage>> {
            Ok(vec![])
        }
    }

    struct StubProcessRunner;
    impl ProcessRunnerPort for StubProcessRunner {
        fn run(
            &self,
            _: &str,
            _: &[&str],
            _: Option<&std::path::Path>,
            _: Option<&[(String, String)]>,
        ) -> anyhow::Result<String> {
            Ok(String::new())
        }
    }

    fn core_with_one_context() -> AgkCore {
        let mut contexts = HashMap::new();
        contexts.insert(
            "personal".to_string(),
            ContextConfig {
                display_name: Some("Personal".to_string()),
                ..Default::default()
            },
        );
        let file = ContextFile {
            current_context: "personal".to_string(),
            contexts,
        };
        // ContextId is still imported (used by ContextStorePort default method
        // signature); keep the import explicit even though we use String here.
        let _ = ContextId::default();

        AgkCore::new(
            Arc::new(StubConfigStore),
            Arc::new(FixedContextStore { file }),
            Arc::new(StubMcp),
            Arc::new(StubVaultSearch),
            Arc::new(Registry::new()),
            HashMap::new(),
            Arc::new(StubProcessRunner),
            std::path::PathBuf::from("."),
        )
    }

    /// Drain the TUI's mpsc channel into the same `SinkCall` shape the CLI
    /// recorder uses. `CoreEvent::Error(_)` events are unwrapped back into
    /// `SinkCall::Error` because `TuiPresenter::on_error` wraps them on the
    /// way out — restoring the call symmetry the assertion relies on.
    fn drain_tui_channel(mut rx: tokio::sync::mpsc::UnboundedReceiver<AppEvent>) -> Vec<SinkCall> {
        let mut out = Vec::new();
        while let Ok(app_event) = rx.try_recv() {
            match app_event {
                AppEvent::CoreEvent(CoreEvent::Error(msg)) => out.push(SinkCall::Error(msg)),
                AppEvent::CoreEvent(e) => out.push(SinkCall::Event(e)),
                other => panic!(
                    "unexpected non-CoreEvent on TuiPresenter channel: {:?}",
                    other
                ),
            }
        }
        out
    }

    #[test]
    fn list_contexts_emits_identical_events_in_both_adapters() {
        let core = core_with_one_context();

        // CLI path: events captured by a direct sink.
        let mut cli = Recorder::default();
        let _ = core.execute(CoreCommand::ListContexts, &mut cli);

        // TUI path: events round-trip through TuiPresenter's mpsc channel.
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut tui_presenter = TuiPresenter::new(tx);
        let _ = core.execute(CoreCommand::ListContexts, &mut tui_presenter);
        drop(tui_presenter);
        let tui_calls = drain_tui_channel(rx);

        assert_eq!(
            cli.calls, tui_calls,
            "CoreEvent / on_error call sequences must be identical across CLI and TUI sinks"
        );
        assert!(
            !cli.calls.is_empty(),
            "Expected ListContexts on a single-context fixture to emit at least one sink call"
        );
    }
}
