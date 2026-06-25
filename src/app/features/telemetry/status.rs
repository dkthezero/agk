use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreResult};
use std::path::Path;

pub fn run(path: &Path, sink: &mut dyn CoreEventSink) -> CoreResult {
    let config = crate::domain::telemetry::AnalyticsConfig::load(path)?;
    let status = crate::domain::telemetry::TelemetryStatus {
        enabled: config.settings.enabled,
        skills_tracked: config.skills.len(),
        templates_tracked: config.templates.len(),
        profiles_tracked: config.profiles.len(),
        last_scan: config.settings.last_scan.clone(),
    };
    sink.on_event(CoreEvent::TelemetryStatusReport(status));
    Ok(crate::app::outcome::CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::outcome::NullSink;

    #[test]
    fn status_missing_analytics_file_reports_default_status() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("does_not_exist.toml");
        let mut sink = NullSink;
        let result = run(&path, &mut sink);
        assert!(
            result.is_ok(),
            "missing analytics file should default, not error"
        );
    }

    #[test]
    fn status_malformed_analytics_file_surfaces_error() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("analytics.toml");
        std::fs::write(&path, "this is = = not valid toml [[[").unwrap();
        let mut sink = NullSink;
        let result = run(&path, &mut sink);
        assert!(
            result.is_err(),
            "malformed analytics file should surface an error"
        );
    }
}
