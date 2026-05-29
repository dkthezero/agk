use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreResult};
use std::path::Path;

pub fn run(path: &Path, sink: &mut dyn CoreEventSink) -> CoreResult {
    let mut config = crate::domain::telemetry::AnalyticsConfig::load(path).unwrap_or_default();
    config.settings.enabled = true;
    config.settings.last_scan = Some(chrono::Utc::now().to_rfc3339());
    config.save(path)?;
    sink.on_event(CoreEvent::TelemetryEnabled);
    Ok(crate::app::outcome::CoreOutcome::Ok)
}
