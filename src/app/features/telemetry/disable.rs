use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreResult};
use std::path::Path;

pub fn run(path: &Path, sink: &mut dyn CoreEventSink) -> CoreResult {
    let mut config = crate::domain::telemetry::AnalyticsConfig::load(path)?;
    config.settings.enabled = false;
    config.settings.last_scan = None;
    config.save(path)?;
    sink.on_event(CoreEvent::TelemetryDisabled);
    Ok(crate::app::outcome::CoreOutcome::Ok)
}
