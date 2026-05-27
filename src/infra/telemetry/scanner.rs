use crate::domain::telemetry::AnalyticsConfig;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::interval;

/// Background telemetry scanner state.
pub struct Scanner {
    pub config_path: PathBuf,
    pub parsers: Vec<Box<dyn crate::infra::telemetry::parser::LogParser>>,
    pub enabled: bool,
}

impl Scanner {
    pub fn new(config_path: PathBuf) -> Self {
        let config = AnalyticsConfig::load(&config_path).unwrap_or_default();
        Self {
            config_path,
            parsers: crate::infra::telemetry::parser::default_parsers(),
            enabled: config.settings.enabled,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
        let _ = std::fs::create_dir_all(
            self.config_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
        );
        let config = AnalyticsConfig {
            settings: crate::domain::telemetry::AnalyticsSettings {
                enabled: true,
                last_scan: Some(chrono::Utc::now().to_rfc3339()),
            },
            ..AnalyticsConfig::default()
        };
        let _ = config.save(&self.config_path);
    }

    pub fn disable(&mut self) {
        self.enabled = false;
        let _ = std::fs::create_dir_all(
            self.config_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
        );
        let config = AnalyticsConfig {
            settings: crate::domain::telemetry::AnalyticsSettings {
                enabled: false,
                last_scan: None,
            },
            ..AnalyticsConfig::default()
        };
        let _ = config.save(&self.config_path);
    }

    pub fn status(&self) -> TelemetryStatus {
        let config = AnalyticsConfig::load(&self.config_path).unwrap_or_default();
        TelemetryStatus {
            enabled: config.settings.enabled,
            skills_tracked: config.skills.len(),
            last_scan: config.settings.last_scan,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TelemetryStatus {
    pub enabled: bool,
    pub skills_tracked: usize,
    pub last_scan: Option<String>,
}

/// Run the background scan loop.
/// Wakes every 60 seconds while enabled.
pub async fn run(scanner: Scanner) {
    let mut timer = interval(Duration::from_secs(60));
    loop {
        timer.tick().await;
        let mut config = AnalyticsConfig::load(&scanner.config_path).unwrap_or_default();
        if !config.settings.enabled {
            continue;
        }
        let scan_started_at = chrono::Utc::now().to_rfc3339();
        for parser in &scanner.parsers {
            crate::infra::telemetry::parser::scan_directory(parser.as_ref(), &mut config);
        }
        config.settings.last_scan = Some(scan_started_at);
        let _ = config.save(&scanner.config_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_loads_existing_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("analytics.toml");
        let mut config = AnalyticsConfig::default();
        config.settings.enabled = true;
        config.save(&path).unwrap();

        let scanner = Scanner::new(path);
        assert!(scanner.enabled);
    }

    #[test]
    fn scanner_enable_creates_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("analytics.toml");
        let mut scanner = Scanner::new(path.clone());
        scanner.enable();
        assert!(path.exists());
        let config = AnalyticsConfig::load(&path).unwrap();
        assert!(config.settings.enabled);
    }

    #[test]
    fn scanner_disable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("analytics.toml");
        let mut scanner = Scanner::new(path.clone());
        scanner.enable();
        scanner.disable();
        let config = AnalyticsConfig::load(&path).unwrap();
        assert!(!config.settings.enabled);
    }
}
