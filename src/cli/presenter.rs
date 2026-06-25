use crate::app::event::CoreEvent;
use crate::cli::presenter_json::event_to_json;
use serde::Serialize;

// CoreEventSink implementation lives in presenter_sink.rs for LOC reasons.

/// CLI presenter: implements [`CoreEventSink`] and formats output as either
/// human-readable text or structured JSON (depending on `--json` flag).
///
/// Quiet mode suppresses non-error output; errors are still written to stderr.
pub struct CliPresenter {
    pub(crate) mode: OutputMode,
    /// Accumulated events for JSON batch output.
    pub(crate) events: Vec<CoreEvent>,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputMode {
    Quiet,
    Normal,
    Json,
}

impl CliPresenter {
    pub fn new(json: bool, quiet: bool) -> Self {
        // `--json` takes precedence over `--quiet`: a user requesting
        // structured JSON output still expects the JSON batch even when they
        // also pass `--quiet` (common pattern: `--quiet --json` to get only
        // machine-readable output with no human-readable noise).  Previously
        // `--quiet` won, making `--quiet --json` emit zero output — silently
        // breaking JSON consumers that paired the two flags.
        let mode = if json {
            OutputMode::Json
        } else if quiet {
            OutputMode::Quiet
        } else {
            OutputMode::Normal
        };
        Self {
            mode,
            events: Vec::new(),
        }
    }

    pub fn mode(&self) -> OutputMode {
        self.mode
    }

    /// Returns true if the presenter has already rendered a failure event
    /// during this execution.  Use cases that emit such an event AND return
    /// `Err` rely on this so the dispatcher's `Err` arm can skip the
    /// redundant `on_error` call (which would otherwise print a second
    /// `Error: ...` line to stderr — see the AGENTS.md anti-pattern note).
    ///
    /// Recognized failure events:
    /// - `TaskFailed` (sync/install/remove/update, mcp registration soft-fail)
    /// - `McpTested { healthy: false }` (mcp connectivity probe)
    /// - `LlmProviderHealth { reachable: false }` (llm health probe)
    /// - `Error` (mcp enable/disable/toggle, asset per-provider errors)
    pub(crate) fn already_reported_task_failure(&self) -> bool {
        self.events.iter().any(|e| {
            matches!(e, CoreEvent::TaskFailed { .. })
                || matches!(e, CoreEvent::McpTested { healthy: false, .. })
                || matches!(
                    e,
                    CoreEvent::LlmProviderHealth { status, .. } if !status.reachable
                )
                || matches!(e, CoreEvent::Error(..))
        })
    }

    /// Render a `CoreEvent::Error` to the human-readable streams.
    ///
    /// In JSON mode the event is already accumulated into the JSON batch by
    /// the sink, so the human-readable line is suppressed here to avoid
    /// duplicate output.  In text/quiet mode the message is written to stderr
    /// so it is visible even when stdout is piped.
    pub(crate) fn render_error_event(&self, message: &str) {
        if matches!(self.mode, OutputMode::Json) {
            return;
        }
        self.eprint(message);
    }

    /// Prints the final JSON batch if `--json`.
    pub fn finalize(&self) {
        if matches!(self.mode, OutputMode::Json) && !self.events.is_empty() {
            let json_events: Vec<serde_json::Value> =
                self.events.iter().map(event_to_json).collect();
            let summary = JsonSummary {
                events: json_events,
            };
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        }
    }

    pub(crate) fn print(&self, msg: &str) {
        if matches!(self.mode, OutputMode::Normal) {
            println!("{}", msg);
        }
    }

    pub(crate) fn eprint(&self, msg: &str) {
        eprintln!("{}", msg);
    }

    /// Render a `ValidationReport` to the human-readable streams.
    ///
    /// In JSON mode the event is already accumulated into the JSON batch by
    /// the sink, so the human-readable line is suppressed here to avoid
    /// duplicate output.  A failing validation is written to stderr so it is
    /// visible even when stdout is piped; a passing one goes to stdout.
    pub(crate) fn render_validation_report(&self, passed: bool, message: &str) {
        if matches!(self.mode, OutputMode::Json) {
            return;
        }
        if passed {
            self.print(&format!("Validation passed: {}", message));
        } else {
            self.eprint(&format!("Validation failed: {}", message));
        }
    }

    /// Render an `on_error` report.
    ///
    /// In JSON mode the error is pushed as a structured `CoreEvent::Error`
    /// into the JSON batch (printed to stdout by `finalize()`), so JSON
    /// consumers get a machine-readable failure signal rather than a
    /// plain-text `Error: ...` line on stderr with an empty stdout batch.
    /// In text/quiet mode the human-readable `Error: ...` line is written to
    /// stderr so it is visible even when stdout is piped.
    pub(crate) fn render_on_error(&mut self, error: String) {
        if matches!(self.mode, OutputMode::Json) {
            self.events.push(CoreEvent::Error(error));
        } else {
            self.eprint(&format!("Error: {}", error));
        }
    }

    /// Render a `TaskFailed` event to the human-readable streams.
    ///
    /// In JSON mode the event is emitted via the batch in `finalize()`, so
    /// the human-readable `[id] Failed: ...` line is suppressed here to
    /// avoid a duplicate stderr line polluting JSON consumers.
    pub(crate) fn render_task_failed(&self, id: usize, error: &str) {
        if matches!(self.mode, OutputMode::Json) {
            // The event is accumulated into the JSON batch by `finalize()`;
            // avoid a duplicate human-readable line.
        } else {
            self.eprint(&format!("[{}] Failed: {}", id, error));
        }
    }

    /// Render a `TaskHungWarning` event to the human-readable streams.
    ///
    /// In JSON mode the event is emitted via the batch in `finalize()`, so
    /// the human-readable `[HUNG] ...` line is suppressed here to avoid a
    /// duplicate stderr line polluting JSON consumers.
    pub(crate) fn render_task_hung_warning(&self, id: usize, name: &str, elapsed_sec: u64) {
        if matches!(self.mode, OutputMode::Json) {
            // The event is accumulated into the JSON batch by `finalize()`;
            // avoid a duplicate human-readable line.
        } else {
            self.eprint(&format!(
                "[HUNG] Task {} '{}' has been running for {}s",
                id, name, elapsed_sec
            ));
        }
    }

    /// Render a `ProfileListed` event to the human-readable streams.
    ///
    /// In JSON mode the event is emitted via the batch in `finalize()`, so
    /// the human-readable lines are suppressed here to avoid duplicates.
    pub(crate) fn render_profile_listed(&self, entries: &[crate::app::snapshot::ProfileEntry]) {
        if matches!(self.mode, OutputMode::Json) {
            // The event is accumulated into the JSON batch by `finalize()`;
            // avoid a duplicate inline print.
        } else if entries.is_empty() {
            self.print("No profiles configured.");
        } else {
            for e in entries {
                let drift = if e.has_drift { " (drift)" } else { "" };
                self.print(&format!(
                    "{} [{}] skills={} mcps={}{}",
                    e.name,
                    e.provider_id,
                    e.skills.len(),
                    e.mcps.len(),
                    drift
                ));
            }
        }
    }

    /// Render a `ContextListed` event to the human-readable streams.
    ///
    /// In JSON mode the event is emitted via the batch in `finalize()`, so
    /// the human-readable lines are suppressed here to avoid duplicates.
    pub(crate) fn render_context_listed(&self, entries: &[crate::app::snapshot::ContextEntry]) {
        if matches!(self.mode, OutputMode::Json) {
            // The event is accumulated into the JSON batch by `finalize()`;
            // avoid a duplicate inline print.
        } else if entries.is_empty() {
            self.print("No contexts configured.");
        } else {
            for e in entries {
                let marker = if e.is_active { "* " } else { "  " };
                let display = e.display_name.as_deref().unwrap_or(&e.name);
                let env = e.environment.as_deref().unwrap_or("");
                self.print(&format!(
                    "{}{} [{}] (vaults: {}, profiles: {})",
                    marker,
                    display,
                    env,
                    e.vaults.len(),
                    e.profiles.len()
                ));
            }
        }
    }

    /// Render an `LlmProviderHealth` event to the human-readable streams.
    ///
    /// In JSON mode the event is emitted via the batch in `finalize()`, so
    /// the human-readable line is suppressed here to avoid a duplicate.
    pub(crate) fn render_llm_health(
        &self,
        id: &str,
        status: &crate::domain::llm_provider::LlmHealthStatus,
    ) {
        if status.reachable {
            self.print(&format!(
                "{} reachable ({} ms)",
                id,
                status.latency_ms.unwrap_or(0)
            ));
        } else if matches!(self.mode, OutputMode::Json) {
            // In JSON mode the event is emitted via the batch in `finalize()`;
            // avoid a duplicate human-readable line.
        } else {
            let reason = status.error.as_deref().unwrap_or("unknown error");
            self.eprint(&format!("{} unreachable: {}", id, reason));
        }
    }
}

/// A serialisable summary of events emitted during a command execution.
#[derive(Serialize)]
pub(crate) struct JsonSummary {
    pub(crate) events: Vec<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::outcome::CoreEventSink;
    use crate::domain::profile::ProfileId;

    #[test]
    fn quiet_mode_suppresses_output() {
        let mut presenter = CliPresenter::new(false, true);
        presenter.on_event(CoreEvent::ProfileCreated(ProfileId::new("test")));
        presenter.on_error("something went wrong".into());
        presenter.finalize();
        // No assertions needed — just must not panic
    }

    #[test]
    fn json_mode_collects_events() {
        let mut presenter = CliPresenter::new(true, false);
        presenter.on_event(CoreEvent::ProfileCreated(ProfileId::new("dev")));
        presenter.on_event(CoreEvent::ProviderActivated("opencode".into()));
        // finalize() would print JSON — we just verify it doesn't panic
        presenter.finalize();
    }

    /// Regression: in `--json` mode, an `Err`-only failure (one that does NOT
    /// emit a structured failure event) must surface as a structured
    /// `CoreEvent::Error` entry in the JSON batch rather than a plain-text
    /// `Error: ...` line on stderr.  Previously `on_error` always wrote
    /// `Error: ...` to stderr, polluting JSON consumers that parse stdout and
    /// leaving the JSON batch empty so callers could not tell the command
    /// failed.
    #[test]
    fn on_error_in_json_mode_pushes_structured_error_event() {
        let mut presenter = CliPresenter::new(true, false);
        presenter.on_error("boom".into());
        assert!(
            presenter.events.iter().any(|e| matches!(
                e,
                CoreEvent::Error(msg) if msg == "boom"
            )),
            "JSON-mode on_error must push a CoreEvent::Error into the batch"
        );
    }

    /// In text mode `on_error` must keep writing `Error: ...` to stderr (it
    /// must NOT push an event, otherwise the human-readable line would be
    /// duplicated via the sink's `Error` render branch).
    #[test]
    fn on_error_in_text_mode_does_not_push_event() {
        let mut presenter = CliPresenter::new(false, false);
        presenter.on_error("boom".into());
        assert!(
            presenter.events.is_empty(),
            "text-mode on_error must not push an event into the batch"
        );
    }

    /// Regression: `CoreEvent::TaskFailed` must be accumulated into the JSON
    /// batch in `--json` mode (so `finalize()` emits it as the sole output)
    /// rather than also leaking a human-readable `[0] Failed: ...` line to
    /// stderr.  Previously the `TaskFailed` (and `TaskHungWarning`) render
    /// branches in `presenter_sink.rs` called `self.eprint` unconditionally,
    /// polluting JSON consumers with a mixed stderr/stdout output even though
    /// the event was already in the batch.  This test locks in that the event
    /// reaches the batch in JSON mode (the human-readable line is suppressed by
    /// `render_task_failed`'s JSON guard).
    #[test]
    fn task_failed_is_in_json_batch_in_json_mode() {
        let mut presenter = CliPresenter::new(true, false);
        presenter.on_event(CoreEvent::TaskFailed {
            id: 0,
            error: "No active providers".into(),
        });
        assert!(
            presenter.events.iter().any(|e| matches!(
                e,
                CoreEvent::TaskFailed { id, error }
                    if *id == 0 && error == "No active providers"
            )),
            "JSON-mode TaskFailed must be accumulated into the batch"
        );
    }

    /// Regression: `--quiet --json` must still emit the JSON batch.  Previously
    /// `--quiet` took precedence over `--json` in `CliPresenter::new`, so the
    /// mode became `Quiet` and `finalize()` (which only prints in `Json` mode)
    /// emitted zero output — silently breaking JSON consumers that paired the
    /// two flags to get machine-readable output with no human-readable noise.
    /// `--json` now takes precedence, so `--quiet --json` behaves like `--json`.
    #[test]
    fn quiet_with_json_still_emits_json_batch() {
        let presenter = CliPresenter::new(true, true);
        assert!(
            matches!(presenter.mode, OutputMode::Json),
            "--json must take precedence over --quiet so the JSON batch is still emitted"
        );
    }
}
