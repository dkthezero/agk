use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreResult};
use std::path::Path;

pub fn run(path: &Path, sink: &mut dyn CoreEventSink) -> CoreResult {
    let config = crate::domain::telemetry::AnalyticsConfig::load(path).unwrap_or_default();
    let status = crate::domain::telemetry::TelemetryStatus {
        enabled: config.settings.enabled,
        skills_tracked: config.skills.len(),
        last_scan: config.settings.last_scan.clone(),
    };
    sink.on_event(CoreEvent::TelemetryStatusReport(status));
    Ok(crate::app::outcome::CoreOutcome::Ok)
}
