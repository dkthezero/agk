use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::profile::ProfileAssetRef;

/// Portable serialization of a profile for cross-machine sharing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedProfile {
    pub agk_version: String,
    pub exported_at: String,
    pub profile: ExportPayload,
}

/// The profile data that is exported/imported, decoupled from internal
/// domain model so the wire format can evolve independently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPayload {
    pub name: String,
    pub provider_id: String,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_answers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub skills: Vec<ProfileAssetRef>,
    #[serde(default)]
    pub mcps: Vec<ProfileAssetRef>,
    #[serde(default)]
    pub instructions: Vec<ProfileAssetRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    pub agent_markdown: String,
}

/// Check version compatibility between an exported profile version and
/// the current agk version.
///
/// - Major version mismatch: returns an error (blocking).
/// - Minor version mismatch: returns Ok but the caller should emit a warning.
/// - Patch difference: always Ok.
pub fn check_version_compatibility(
    export_version: &str,
    current_version: &str,
) -> Result<(), String> {
    let export_parts: Vec<&str> = export_version.split('.').collect();
    let current_parts: Vec<&str> = current_version.split('.').collect();

    if export_parts.len() < 2 || current_parts.len() < 2 {
        return Err(format!(
            "Cannot parse version numbers: export={}, current={}",
            export_version, current_version
        ));
    }

    let export_major: u32 = export_parts[0]
        .parse()
        .map_err(|_| format!("Cannot parse major version from export: {}", export_version))?;
    let current_major: u32 = current_parts[0].parse().map_err(|_| {
        format!(
            "Cannot parse major version from current: {}",
            current_version
        )
    })?;

    if export_major != current_major {
        return Err(format!(
            "Major version mismatch: export={} vs current={}. The profile may not be compatible.",
            export_version, current_version
        ));
    }

    let export_minor: u32 = export_parts[1]
        .parse()
        .map_err(|_| format!("Cannot parse minor version from export: {}", export_version))?;
    let current_minor: u32 = current_parts[1].parse().map_err(|_| {
        format!(
            "Cannot parse minor version from current: {}",
            current_version
        )
    })?;

    if export_minor != current_minor {
        return Ok(()); // Caller should emit a warning
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_profile_json_roundtrip() {
        let payload = ExportPayload {
            name: "dev".to_string(),
            provider_id: "opencode".to_string(),
            scope: "workspace".to_string(),
            structured_answers: None,
            skills: vec![ProfileAssetRef::new("rust", "auto")],
            mcps: vec![ProfileAssetRef::new("github", "auto")],
            instructions: vec![],
            tools: vec!["Read".to_string(), "Glob".to_string()],
            permission_mode: Some("auto".to_string()),
            agent_markdown: "# Dev Agent\nYou are a dev agent.".to_string(),
        };
        let exported = ExportedProfile {
            agk_version: "0.2.7".to_string(),
            exported_at: "2026-06-01T00:00:00Z".to_string(),
            profile: payload,
        };
        let json = serde_json::to_string(&exported).unwrap();
        let deserialized: ExportedProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.agk_version, "0.2.7");
        assert_eq!(deserialized.profile.name, "dev");
        assert_eq!(deserialized.profile.skills.len(), 1);
        assert_eq!(deserialized.profile.tools.len(), 2);
        assert_eq!(
            deserialized.profile.agent_markdown,
            "# Dev Agent\nYou are a dev agent."
        );
    }

    #[test]
    fn version_compatibility_major_mismatch_is_error() {
        let result = check_version_compatibility("1.0.0", "0.2.7");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Major version mismatch"));
    }

    #[test]
    fn version_compatibility_minor_mismatch_is_ok() {
        let result = check_version_compatibility("0.1.0", "0.2.7");
        assert!(result.is_ok());
    }

    #[test]
    fn version_compatibility_same_major_is_ok() {
        let result = check_version_compatibility("0.2.0", "0.2.7");
        assert!(result.is_ok());
    }

    #[test]
    fn version_compatibility_unparseable_is_error() {
        let result = check_version_compatibility("abc", "0.2.7");
        assert!(result.is_err());
    }

    #[test]
    fn version_compatibility_malformed_minor_is_error() {
        let result = check_version_compatibility("0.x.0", "0.2.7");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Cannot parse minor version from export"));
    }

    #[test]
    fn version_compatibility_malformed_current_minor_is_error() {
        let result = check_version_compatibility("0.2.0", "0.y.7");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Cannot parse minor version from current"));
    }

    #[test]
    fn export_payload_skips_empty_collections() {
        let payload = ExportPayload {
            name: "minimal".to_string(),
            provider_id: "opencode".to_string(),
            scope: "workspace".to_string(),
            structured_answers: None,
            skills: vec![],
            mcps: vec![],
            instructions: vec![],
            tools: vec![],
            permission_mode: None,
            agent_markdown: "".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        // tools should be skipped (empty vec with skip_serializing_if)
        assert!(!json.contains("\"tools\""));
        // structured_answers and permission_mode should be skipped (None)
        assert!(!json.contains("\"structured_answers\""));
        assert!(!json.contains("\"permission_mode\""));
    }
}
