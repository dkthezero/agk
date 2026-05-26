use super::*;
use crate::cli::entry::{Cli, TelemetryCommands};
use anyhow::Result;

pub fn dispatch_telemetry(cli: &Cli, command: &TelemetryCommands) -> Result<i32> {
    match command {
        TelemetryCommands::Enable => {
            let path = crate::domain::paths::analytics_path();
            let mut scanner = crate::infra::telemetry::scanner::Scanner::new(path);
            scanner.enable();
            println_if_not_quiet(
                &OutputMode::from_cli(cli),
                "Telemetry enabled. Background scanner started.",
            );
            Ok(EXIT_SUCCESS)
        }
        TelemetryCommands::Disable => {
            let path = crate::domain::paths::analytics_path();
            let mut scanner = crate::infra::telemetry::scanner::Scanner::new(path);
            scanner.disable();
            println_if_not_quiet(&OutputMode::from_cli(cli), "Telemetry disabled.");
            Ok(EXIT_SUCCESS)
        }
        TelemetryCommands::Status => {
            let path = crate::domain::paths::analytics_path();
            let scanner = crate::infra::telemetry::scanner::Scanner::new(path);
            let status = scanner.status();
            let mode = OutputMode::from_cli(cli);
            if matches!(mode, OutputMode::Json) {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println_if_not_quiet(
                    &mode,
                    &format!(
                        "Telemetry: {} | Skills tracked: {} | Last scan: {}",
                        if status.enabled {
                            "enabled"
                        } else {
                            "disabled"
                        },
                        status.skills_tracked,
                        status.last_scan.as_deref().unwrap_or("never")
                    ),
                );
            }
            Ok(EXIT_SUCCESS)
        }
        TelemetryCommands::Export { format, output } => {
            let path = crate::domain::paths::analytics_path();
            let config = crate::domain::telemetry::AnalyticsConfig::load(&path)?;
            let mode = OutputMode::from_cli(cli);

            let content = match format {
                crate::cli::entry::ExportFormat::Json => serde_json::to_string_pretty(&config)?,
                crate::cli::entry::ExportFormat::Csv => telemetry_to_csv(&config),
            };

            if let Some(file_path) = output {
                std::fs::write(file_path, &content)?;
                println_if_not_quiet(&mode, &format!("Telemetry exported to {}", file_path));
            } else {
                println!("{}", content);
            }
            Ok(EXIT_SUCCESS)
        }
    }
}
