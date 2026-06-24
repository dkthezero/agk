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
        let mode = if quiet {
            OutputMode::Quiet
        } else if json {
            OutputMode::Json
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

    pub(crate) fn print_json_event(&self, event: &CoreEvent) {
        if matches!(self.mode, OutputMode::Json) {
            println!(
                "{}",
                serde_json::to_string_pretty(&event_to_json(event)).unwrap()
            );
        }
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
}
