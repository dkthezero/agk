use crate::domain::telemetry::AnalyticsConfig;
use anyhow::Result;

/// Port for analytics/telemetry config storage. The concrete `FileTelemetryStore`
/// in `infra/telemetry/store.rs` reads and writes the analytics.toml file.
pub trait TelemetryStorePort: Send + Sync {
    fn load(&self, path: &std::path::Path) -> Result<AnalyticsConfig>;
    fn save(&self, path: &std::path::Path, config: &AnalyticsConfig) -> Result<()>;
}
