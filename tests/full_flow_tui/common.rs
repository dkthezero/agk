use agk::app::command::CoreCommand;
use agk::app::core::AgkCore;
use agk::app::event::CoreEvent;
use agk::app::outcome::{CoreEventSink, CoreResult};
use agk::app::ports::{ProviderPort, VaultSearchPort};
use agk::app::registry::Registry;
use agk::domain::asset::{AssetKind, ScannedPackage};
use agk::domain::config::ConfigFile;
use agk::domain::identity::AssetIdentity;
use agk::domain::scope::Scope;
use agk::tui::app::AppState;
use agk::tui::core_event_reducer::apply_core_event;
use agk::tui::render::draw;
use anyhow::Result;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

struct FakeProvider;
impl ProviderPort for FakeProvider {
    fn id(&self) -> &str {
        "opencode"
    }
    fn name(&self) -> &str {
        "OpenCode"
    }
    fn install(&self, _: &ScannedPackage, _: Scope, _: Option<&ConfigFile>, _: bool) -> Result<()> {
        Ok(())
    }
    fn remove(
        &self,
        _: &AssetIdentity,
        _: &AssetKind,
        _: Scope,
        _: Option<&ConfigFile>,
    ) -> Result<()> {
        Ok(())
    }
    fn supports_profiles(&self) -> bool {
        true
    }
}

struct FakeVaultSearch;
#[async_trait::async_trait]
impl VaultSearchPort for FakeVaultSearch {
    fn vault_id(&self) -> &str {
        "fake"
    }
    async fn search(&self, _query: &str) -> Result<Vec<ScannedPackage>> {
        Ok(vec![])
    }
}

// ---------------------------------------------------------------------------
// Core builder
// ---------------------------------------------------------------------------

pub fn test_core() -> AgkCore {
    let mut registry = Registry::new();
    registry.register_provider(Box::new(FakeProvider));
    AgkCore::new(
        Arc::new(agk::app::test_support::FakeStore::new()),
        Arc::new(agk::app::test_support::FakeContextStore::new()),
        Arc::new(agk::app::test_support::FakeMcpRegistry::new()),
        Arc::new(FakeVaultSearch),
        Arc::new(registry),
        HashMap::new(),
        Arc::new(agk::app::test_support::FakeProcessRunner::new()),
        Arc::new(agk::infra::task_tracker::InMemoryTaskTracker::new()),
        std::path::PathBuf::from("."),
        Arc::new(agk::app::test_support::FakeClawHub::new()),
    )
}

// ---------------------------------------------------------------------------
// Sink that applies CoreEvents directly into AppState (bypasses async channel)
// ---------------------------------------------------------------------------

pub struct StateSink<'a> {
    pub state: &'a mut AppState,
}

impl<'a> CoreEventSink for StateSink<'a> {
    fn on_event(&mut self, event: CoreEvent) {
        apply_core_event(self.state, &event);
    }

    fn on_error(&mut self, error: String) {
        self.state.status_line = format!("Error: {}", error);
    }
}

pub fn execute(core: &AgkCore, state: &mut AppState, cmd: CoreCommand) -> CoreResult {
    let mut sink = StateSink { state };
    let result = core.execute(cmd, &mut sink);
    if let Err(ref e) = result {
        sink.state.status_line = format!("Error: {}", e);
    }
    result
}

// ---------------------------------------------------------------------------
// Rendering helper
// ---------------------------------------------------------------------------

pub fn render_buffer(state: &AppState, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| draw(frame, state)).unwrap();
    terminal.backend().buffer().clone()
}

/// Assert that the buffer contains a given substring somewhere.
pub fn assert_buffer_contains(buf: &Buffer, needle: &str) {
    let text: String = buf.content.iter().map(|cell| cell.symbol()).collect();
    assert!(
        text.contains(needle),
        "Expected buffer to contain '{}'. Buffer text:\n{}",
        needle,
        text
    );
}
