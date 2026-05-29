use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreResult};
use crate::domain::telemetry::{AnalyticsConfig, TelemetryExportFormat};
use std::path::Path;

pub fn run(
    path: &Path,
    format: TelemetryExportFormat,
    output_path: Option<String>,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let config = AnalyticsConfig::load(path).unwrap_or_default();

    let content = match format {
        TelemetryExportFormat::Json => serde_json::to_string_pretty(&config)?,
        TelemetryExportFormat::Csv => telemetry_to_csv(&config),
    };

    sink.on_event(CoreEvent::TelemetryExported {
        content: content.clone(),
        output_path: output_path.clone(),
    });

    Ok(crate::app::outcome::CoreOutcome::Ok)
}

fn telemetry_to_csv(config: &AnalyticsConfig) -> String {
    let mut lines = vec!["skill,invocations,last_used,providers".to_string()];
    for (name, analytics) in &config.skills {
        let last = analytics.last_used.as_deref().unwrap_or("never");
        let providers = analytics.providers().join("; ");
        lines.push(format!(
            "\"{}\",{},\"{}\",\"{}\"",
            name.replace('"', "\"\""),
            analytics.total_invocations,
            last,
            providers.replace('"', "\"\""),
        ));
    }
    lines.join("\n")
}
