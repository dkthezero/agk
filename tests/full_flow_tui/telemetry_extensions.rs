//! Integration tests for Telemetry Extensions (F19/F21/F22).
//!
//! Tests that:
//! - TemplateAnalytics and ProfileAnalytics serialize/deserialize correctly
//! - increment_template_selection updates counters
//! - increment_profile_launch updates counters
//! - Backward-compatible deserialization (old analytics.toml without new fields)
//! - CSV export format is correct

use agk::domain::telemetry::{AnalyticsConfig, ProfileAnalytics, TemplateAnalytics};

#[test]
fn template_analytics_serde_roundtrip() {
    let ta = TemplateAnalytics {
        selections: 15,
        last_selected: Some("2026-06-01T09:00:00Z".to_string()),
    };
    let toml_text = toml::to_string(&ta).unwrap();
    let parsed: TemplateAnalytics = toml::from_str(&toml_text).unwrap();
    assert_eq!(parsed.selections, 15);
    assert_eq!(parsed.last_selected, Some("2026-06-01T09:00:00Z".to_string()));
}

#[test]
fn profile_analytics_serde_roundtrip() {
    let pa = ProfileAnalytics {
        launches: 23,
        last_launched: Some("2026-06-01T16:45:00Z".to_string()),
        provider: Some("opencode".to_string()),
    };
    let toml_text = toml::to_string(&pa).unwrap();
    let parsed: ProfileAnalytics = toml::from_str(&toml_text).unwrap();
    assert_eq!(parsed.launches, 23);
    assert_eq!(parsed.provider, Some("opencode".to_string()));
}

#[test]
fn increment_template_selection_updates_counters() {
    let mut config = AnalyticsConfig::default();
    config.increment_template_selection("feature-implementer");
    config.increment_template_selection("feature-implementer");
    config.increment_template_selection("code-reviewer");

    let fi = config.templates.get("feature-implementer").unwrap();
    assert_eq!(fi.selections, 2);
    assert!(fi.last_selected.is_some());

    let cr = config.templates.get("code-reviewer").unwrap();
    assert_eq!(cr.selections, 1);
    assert!(cr.last_selected.is_some());
}

#[test]
fn increment_profile_launch_updates_counters() {
    let mut config = AnalyticsConfig::default();
    config.increment_profile_launch("web-app-team", "opencode");
    config.increment_profile_launch("web-app-team", "opencode");
    config.increment_profile_launch("backend-api", "claude-code");

    let wat = config.profiles.get("web-app-team").unwrap();
    assert_eq!(wat.launches, 2);
    assert_eq!(wat.provider, Some("opencode".to_string()));
    assert!(wat.last_launched.is_some());

    let ba = config.profiles.get("backend-api").unwrap();
    assert_eq!(ba.launches, 1);
    assert_eq!(ba.provider, Some("claude-code".to_string()));
}

#[test]
fn backward_compatible_deserialization_without_new_fields() {
    // Old analytics.toml without [templates] or [profiles] sections
    let toml_text = r#"
[settings]
enabled = true
last_scan = "2026-05-01T14:32:00Z"

[skills."web-browsing-tool"]
total_invocations = 42
last_used = "2026-05-01T14:32:00Z"
"#;

    let config: AnalyticsConfig = toml::from_str(toml_text).unwrap();
    assert!(config.settings.enabled);
    assert!(config.templates.is_empty());
    assert!(config.profiles.is_empty());
    assert_eq!(config.skills.len(), 1);
}

#[test]
fn analytics_config_full_roundtrip_with_new_fields() {
    let mut config = AnalyticsConfig::default();
    config.settings.enabled = true;
    config.settings.last_scan = Some("2026-06-01T00:00:00Z".to_string());
    config.increment_template_selection("feature-implementer");
    config.increment_template_selection("code-reviewer");
    config.increment_profile_launch("web-app-team", "opencode");
    config.increment_invocation("web-browsing-tool", "claude-code");

    let toml_text = toml::to_string_pretty(&config).unwrap();
    let parsed: AnalyticsConfig = toml::from_str(&toml_text).unwrap();

    assert_eq!(parsed.templates.len(), 2);
    assert_eq!(parsed.profiles.len(), 1);
    assert_eq!(parsed.skills.len(), 1);
    assert!(parsed.settings.enabled);
}

#[test]
fn csv_export_includes_templates_and_profiles() {
    use agk::app::features::telemetry::export::telemetry_to_csv;

    let mut config = AnalyticsConfig::default();
    config.settings.enabled = true;
    config.increment_invocation("web-browsing-tool", "claude-code");
    config.increment_template_selection("feature-implementer");
    config.increment_profile_launch("web-app-team", "opencode");

    let csv = telemetry_to_csv(&config);

    // Header
    assert!(csv.contains("category,name,count,last_used,providers"));
    // Skill row (names are quoted in CSV)
    assert!(csv.contains("skill,") && csv.contains("web-browsing-tool"));
    // Template row
    assert!(csv.contains("template,") && csv.contains("feature-implementer"));
    // Profile row
    assert!(csv.contains("profile,") && csv.contains("web-app-team"));
}

#[test]
fn csv_export_empty_config() {
    use agk::app::features::telemetry::export::telemetry_to_csv;

    let config = AnalyticsConfig::default();
    let csv = telemetry_to_csv(&config);

    // Should have header only
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(csv.contains("category,name,count,last_used,providers"));
}