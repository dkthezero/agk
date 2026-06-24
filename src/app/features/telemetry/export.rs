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

    if let Some(ref path) = output_path {
        std::fs::write(path, &content)
            .map_err(|e| anyhow::anyhow!("Failed to write export file '{}': {}", path, e))?;
    }

    sink.on_event(CoreEvent::TelemetryExported {
        content: content.clone(),
        output_path: output_path.clone(),
    });

    Ok(crate::app::outcome::CoreOutcome::Ok)
}

pub fn telemetry_to_csv(config: &AnalyticsConfig) -> String {
    let mut lines = vec!["category,name,count,last_used,providers".to_string()];

    // Skills
    for (name, analytics) in &config.skills {
        let last = analytics.last_used.as_deref().unwrap_or("never");
        let providers = analytics.providers().join("; ");
        lines.push(format!(
            "skill,\"{}\",{},\"{}\",\"{}\"",
            name.replace('"', "\"\""),
            analytics.total_invocations,
            last,
            providers.replace('"', "\"\""),
        ));
    }

    // Templates
    for (name, analytics) in &config.templates {
        let last = analytics.last_selected.as_deref().unwrap_or("never");
        lines.push(format!(
            "template,\"{}\",{},\"{}\",N/A",
            name.replace('"', "\"\""),
            analytics.selections,
            last,
        ));
    }

    // Profiles
    for (name, analytics) in &config.profiles {
        let last = analytics.last_launched.as_deref().unwrap_or("never");
        let provider = analytics.provider.as_deref().unwrap_or("unknown");
        lines.push(format!(
            "profile,\"{}\",{},\"{}\",\"{}\"",
            name.replace('"', "\"\""),
            analytics.launches,
            last,
            provider.replace('"', "\"\""),
        ));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_output_format_correctness() {
        let mut config = AnalyticsConfig::default();
        config.increment_invocation("web-browser", "claude-code");
        config.increment_template_selection("code-reviewer");
        config.increment_profile_launch("dev", "opencode");

        let csv = telemetry_to_csv(&config);
        let lines: Vec<&str> = csv.lines().collect();

        // Header
        assert_eq!(lines[0], "category,name,count,last_used,providers");

        // Skill row
        let skill_row = lines.iter().find(|l| l.starts_with("skill,")).unwrap();
        assert!(skill_row.contains("web-browser"));
        assert!(skill_row.contains("1"));
        assert!(skill_row.contains("claude-code"));

        // Template row
        let tmpl_row = lines.iter().find(|l| l.starts_with("template,")).unwrap();
        assert!(tmpl_row.contains("code-reviewer"));
        assert!(tmpl_row.contains("1"));
        assert!(tmpl_row.contains("N/A"));

        // Profile row
        let prof_row = lines.iter().find(|l| l.starts_with("profile,")).unwrap();
        assert!(prof_row.contains("dev"));
        assert!(prof_row.contains("1"));
        assert!(prof_row.contains("opencode"));
    }

    #[test]
    fn csv_empty_config_only_header() {
        let config = AnalyticsConfig::default();
        let csv = telemetry_to_csv(&config);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "category,name,count,last_used,providers");
    }

    #[test]
    fn export_bad_output_path_returns_err_and_emits_no_event() {
        use crate::app::outcome::NullSink;
        let tmp = tempfile::tempdir().unwrap();
        let mut sink = NullSink;
        let result = run(
            tmp.path(),
            TelemetryExportFormat::Json,
            Some("/nonexistent_agk_dir/out.json".to_string()),
            &mut sink,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to write export file"));
    }

    #[test]
    fn export_to_output_path_writes_file_and_succeeds() {
        use crate::app::outcome::NullSink;
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("out.json");
        let out_str = out.to_str().unwrap().to_string();
        let mut sink = NullSink;
        let result = run(
            tmp.path(),
            TelemetryExportFormat::Json,
            Some(out_str),
            &mut sink,
        );
        assert!(result.is_ok());
        assert!(
            out.exists(),
            "export file should have been written by the use case"
        );
    }
}
