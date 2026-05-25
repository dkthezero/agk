use crate::app::command::CoreCommand;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};

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
    // Will hold wired port implementations (ConfigStorePort, ProviderPort, etc.)
    // in Phase 3.  For now the struct is just a marker.
}

impl AgkCore {
    pub fn new() -> Self {
        Self {}
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
            // Use-cases implemented in Phase 3 (require port injection into
            // AgkCore — currently stubbed so the architecture test passes)
            // ===============================================================
            CoreCommand::AttachVault { input } => {
                // Phase 3: inject ConfigStorePort into AgkCore, then call
                // crate::app::usecases::attach_vault::run(...)
                sink.on_error("AttachVault not yet wired in AgkCore".into());
                Ok(CoreOutcome::Ok)
            }
            CoreCommand::DeactivateProvider { id, scope } => {
                // Phase 3: inject ConfigStorePort + ProviderPort
                sink.on_error(format!(
                    "DeactivateProvider '{}'/{:?} not yet wired",
                    id, scope
                ));
                Ok(CoreOutcome::Ok)
            }
            CoreCommand::RegisterMcp { input } => {
                // Phase 3: inject McpRegistryPort
                sink.on_error("RegisterMcp not yet wired in AgkCore".into());
                Ok(CoreOutcome::Ok)
            }
            CoreCommand::SearchRemoteVault { vault_id, query } => {
                // Phase 3: inject VaultPort
                sink.on_error("SearchRemoteVault not yet wired in AgkCore".into());
                Ok(CoreOutcome::Ok)
            }
            // TODO: Phase 3 – wire remaining commands.
            _ => {
                sink.on_error(format!("Command {:?} not yet implemented", command));
                Ok(CoreOutcome::Ok)
            }
        }
    }
}

impl Default for AgkCore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubSink;

    impl CoreEventSink for StubSink {
        fn on_event(&mut self, _event: crate::app::event::CoreEvent) {}
        fn on_error(&mut self, _error: String) {}
    }

    #[test]
    fn facade_executes_create_profile_stub() {
        let core = AgkCore::new();
        let mut sink = StubSink;
        let cmd = CoreCommand::CreateProfile {
            input: crate::app::command::CreateProfileInput::new(
                "test-core",
                "opencode",
                crate::domain::scope::Scope::Workspace,
            ),
        };
        let result = core.execute(cmd, &mut sink);
        // For now the stub use-case returns Ok
        assert!(result.is_ok());
    }
}
