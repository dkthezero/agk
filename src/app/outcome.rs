/// Higher-level outcome wrapper returned by [`crate::app::core::AgkCore::execute`].
///
/// Use cases that produce a single atomic result (e.g. creating a profile,
/// validating config) return `Outcome`.  Use cases that stream progress
/// over time (e.g. sync, refresh) emit [`crate::app::event::CoreEvent`]s via
/// the event sink instead.
/// NOTE: Outcomes are returned incrementally as use-cases are wired into core.rs.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CoreOutcome {
    /// Command succeeded with no additional payload.
    Ok,
    /// A list of profiles was returned.
    Profiles(Vec<crate::domain::profile::Profile>),
    /// A concrete launch plan for `--dry-run`.
    LaunchPlan(crate::app::event::LaunchPlan),
    /// A workspace snapshot (used by `LoadWorkspaceSnapshot`).
    WorkspaceSnapshot(crate::app::event::WorkspaceSnapshot),
    /// A validation report.
    ValidationReport { passed: bool, message: String },
}

impl From<crate::app::event::LaunchPlan> for CoreOutcome {
    fn from(plan: crate::app::event::LaunchPlan) -> Self {
        CoreOutcome::LaunchPlan(plan)
    }
}

impl From<crate::app::event::WorkspaceSnapshot> for CoreOutcome {
    fn from(snapshot: crate::app::event::WorkspaceSnapshot) -> Self {
        CoreOutcome::WorkspaceSnapshot(snapshot)
    }
}

pub type CoreResult = anyhow::Result<CoreOutcome>;

/// Trait for anything that can receive a stream of [`crate::app::event::CoreEvent`]s.
/// The TUI presenter and CLI presenter both implement this.
pub trait CoreEventSink: Send {
    fn on_event(&mut self, event: crate::app::event::CoreEvent);
    fn on_error(&mut self, error: String);
}

/// A no-op sink useful for headless fire-and-forget commands.
#[allow(dead_code)] // test-only no-op sink, used by app/usecases/ tests
pub struct NullSink;

impl CoreEventSink for NullSink {
    fn on_event(&mut self, _event: crate::app::event::CoreEvent) {}
    fn on_error(&mut self, _error: String) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)] // test helper pattern — used by downstream use-case tests
    struct StubSink {
        events: Vec<crate::app::event::CoreEvent>,
    }

    impl CoreEventSink for StubSink {
        fn on_event(&mut self, event: crate::app::event::CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, _error: String) {}
    }

    #[test]
    fn null_sink_does_nothing() {
        let mut sink = NullSink;
        sink.on_event(crate::app::event::CoreEvent::VaultAttached("test".into()));
        // no assertion needed — just must compile and not panic
    }

    #[test]
    fn outcome_from_launch_plan() {
        let plan = crate::app::event::LaunchPlan::default();
        let out: CoreOutcome = plan.into();
        assert!(matches!(out, CoreOutcome::LaunchPlan(_)));
    }
}
