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
    pub templates_tracked: usize,
    pub profiles_tracked: usize,
    pub last_scan: Option<String>,
}

/// Telemetry configuration and data stored in ~/.config/agk/analytics.toml
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    #[serde(default)]
    pub settings: AnalyticsSettings,
    #[serde(default)]
    pub skills: HashMap<String, SkillAnalytics>,
    /// Template usage tracking: template name → analytics.
    #[serde(default)]
    pub templates: HashMap<String, TemplateAnalytics>,
    /// Profile launch tracking: profile name → analytics.
    #[serde(default)]
    pub profiles: HashMap<String, ProfileAnalytics>,
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

/// Analytics for a single template selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateAnalytics {
    pub selections: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_selected: Option<String>,
}

/// Analytics for a single profile launch.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileAnalytics {
    pub launches: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_launched: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl AnalyticsConfig {
    pub fn increment_invocation(&mut self, skill_name: &str, provider_id: &str) {
        let entry = self.skills.entry(skill_name.to_string()).or_default();
        entry.total_invocations += 1;
        entry.last_used = Some(chrono::Utc::now().to_rfc3339());
        entry.increment_provider(provider_id);
    }

    pub fn increment_template_selection(&mut self, template_name: &str) {
        let entry = self.templates.entry(template_name.to_string()).or_default();
        entry.selections += 1;
        entry.last_selected = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn increment_profile_launch(&mut self, profile_name: &str, provider_id: &str) {
        let entry = self.profiles.entry(profile_name.to_string()).or_default();
        entry.launches += 1;
        entry.last_launched = Some(chrono::Utc::now().to_rfc3339());
        entry.provider = Some(provider_id.to_string());
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

    #[test]
    fn increment_template_selection_updates_counters() {
        let mut config = AnalyticsConfig::default();
        config.increment_template_selection("code-reviewer");
        config.increment_template_selection("code-reviewer");
        config.increment_template_selection("feature-implementer");

        let tmpl = config.templates.get("code-reviewer").unwrap();
        assert_eq!(tmpl.selections, 2);
        assert!(tmpl.last_selected.is_some());

        let tmpl2 = config.templates.get("feature-implementer").unwrap();
        assert_eq!(tmpl2.selections, 1);
        assert!(tmpl2.last_selected.is_some());
    }

    #[test]
    fn increment_profile_launch_updates_counters() {
        let mut config = AnalyticsConfig::default();
        config.increment_profile_launch("dev", "opencode");
        config.increment_profile_launch("dev", "opencode");
        config.increment_profile_launch("prod", "claude-code");

        let prof = config.profiles.get("dev").unwrap();
        assert_eq!(prof.launches, 2);
        assert!(prof.last_launched.is_some());
        assert_eq!(prof.provider.as_deref(), Some("opencode"));

        let prof2 = config.profiles.get("prod").unwrap();
        assert_eq!(prof2.launches, 1);
        assert_eq!(prof2.provider.as_deref(), Some("claude-code"));
    }

    #[test]
    fn backward_compatible_deserialization_without_new_fields() {
        // Old analytics.toml that doesn't have templates/profiles
        let old_toml = r#"
[settings]
enabled = true

[skills.web-browser]
total_invocations = 5
last_used = "2025-01-01T00:00:00+00:00"
"#;
        let config: AnalyticsConfig = toml::from_str(old_toml).unwrap();
        assert!(config.settings.enabled);
        assert!(config.templates.is_empty());
        assert!(config.profiles.is_empty());
        assert_eq!(config.skills.get("web-browser").unwrap().total_invocations, 5);
    }

    #[test]
    fn template_analytics_serde_roundtrip() {
        let mut config = AnalyticsConfig::default();
        config.increment_template_selection("code-reviewer");

        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: AnalyticsConfig = toml::from_str(&serialized).unwrap();

        let tmpl = deserialized.templates.get("code-reviewer").unwrap();
        assert_eq!(tmpl.selections, 1);
        assert!(tmpl.last_selected.is_some());
    }
}
