use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Target format for telemetry data export.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TelemetryExportFormat {
    Json,
    Csv,
}

/// Snapshot of telemetry scanner state suitable for event emission.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryStatus {
    pub enabled: bool,
    pub skills_tracked: usize,
    pub last_scan: Option<String>,
}

/// Telemetry configuration and data stored in ~/.config/agk/analytics.toml
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    #[serde(default)]
    pub settings: AnalyticsSettings,
    #[serde(default)]
    pub skills: HashMap<String, SkillAnalytics>,
    /// Per-file byte offsets for deduplication: path → last processed file size in bytes.
    #[serde(default)]
    pub file_offsets: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSettings {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub last_scan: Option<String>,
}

impl Default for AnalyticsSettings {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            last_scan: None,
        }
    }
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillAnalytics {
    pub total_invocations: u64,
    pub last_used: Option<String>,
    #[serde(default)]
    pub provider_counts: HashMap<String, u64>,
    // Legacy field: kept for backward-compatible deserialization.
    #[serde(default, skip_serializing)]
    providers: Vec<String>,
}

impl SkillAnalytics {
    /// Return the list of provider IDs that have recorded invocations.
    pub fn providers(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.provider_counts.keys().cloned().collect();
        // Merge any legacy providers that might not have counts yet
        for p in &self.providers {
            if !keys.contains(p) {
                keys.push(p.clone());
            }
        }
        keys
    }

    /// Increment count for a specific provider.
    pub fn increment_provider(&mut self, provider_id: &str) {
        *self
            .provider_counts
            .entry(provider_id.to_string())
            .or_insert(0) += 1;
    }

    /// Get per-provider breakdown formatted for display.
    pub fn provider_breakdown(&self) -> String {
        let mut parts: Vec<String> = self
            .provider_counts
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect();
        for p in &self.providers {
            if !self.provider_counts.contains_key(p) {
                parts.push(format!("{}: ?", p));
            }
        }
        parts.join(", ")
    }
}

impl AnalyticsConfig {
    pub fn increment_invocation(&mut self, skill_name: &str, provider_id: &str) {
        let entry = self.skills.entry(skill_name.to_string()).or_default();
        entry.total_invocations += 1;
        entry.last_used = Some(chrono::Utc::now().to_rfc3339());
        entry.increment_provider(provider_id);
    }
}

// File-backed `AnalyticsConfig::{load, save}` inherent impls were moved to
// `infra/telemetry/store.rs` by ADR-001 Commit 1 to keep the domain pure.
// The `round_trip` integration test moved alongside the impl.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_enabled() {
        let config = AnalyticsConfig::default();
        assert!(config.settings.enabled);
    }

    #[test]
    fn increment_invocation_updates_counters() {
        let mut config = AnalyticsConfig::default();
        config.increment_invocation("web-browser", "claude-code");
        config.increment_invocation("web-browser", "claude-code");
        config.increment_invocation("web-browser", "opencode");

        let skill = config.skills.get("web-browser").unwrap();
        assert_eq!(skill.total_invocations, 3);
        assert_eq!(skill.provider_counts.get("claude-code"), Some(&2));
        assert_eq!(skill.provider_counts.get("opencode"), Some(&1));
    }
}
