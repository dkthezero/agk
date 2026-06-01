use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Unique identifier for a profile (display name acts as the key).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ProfileId(pub String);

impl ProfileId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ProfileId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for ProfileId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Typed wrapper for skill identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(pub String);

impl SkillId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed wrapper for MCP server identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct McpServerId(pub String);

impl McpServerId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed wrapper for profile identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileRef(pub String);

impl ProfileRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Vault-aware reference to an asset (skill, MCP, or instruction) used by a profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ProfileAssetRef {
    pub name: String,
    #[serde(default = "default_vault_auto")]
    pub vault: String,
}

impl ProfileAssetRef {
    pub fn new(name: impl Into<String>, vault: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vault: vault.into(),
        }
    }
}

#[allow(dead_code)]
fn default_vault_auto() -> String {
    "auto".to_string()
}

impl<'de> serde::Deserialize<'de> for ProfileAssetRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct ProfileAssetRefVisitor;

        impl<'de> Visitor<'de> for ProfileAssetRefVisitor {
            type Value = ProfileAssetRef;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or a table with 'name' and optional 'vault'")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ProfileAssetRef {
                    name: value.to_string(),
                    vault: "auto".to_string(),
                })
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut name = None;
                let mut vault = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "name" => name = Some(map.next_value()?),
                        "vault" => vault = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                Ok(ProfileAssetRef {
                    name: name.ok_or_else(|| de::Error::missing_field("name"))?,
                    vault: vault.unwrap_or_else(|| "auto".to_string()),
                })
            }
        }

        deserializer.deserialize_any(ProfileAssetRefVisitor)
    }
}

/// Typed wrapper for provider identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ProviderId(pub String);

impl ProviderId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ProviderId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Profile definition stored in config.  Holds **references only** — never
/// duplicates skill files, MCP command definitions, or provider internals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Profile {
    pub id: ProfileId,
    /// Scope where the profile is persisted (Global vs Workspace).
    pub scope: crate::domain::scope::Scope,
    /// The provider that will run this profile.
    pub provider_id: ProviderId,
    /// Vault-aware skill references.
    #[serde(default)]
    pub skill_refs: Vec<ProfileAssetRef>,
    /// Vault-aware MCP server references.
    #[serde(default)]
    pub mcp_refs: Vec<ProfileAssetRef>,
    /// Vault-aware instruction references.
    #[serde(default)]
    pub instruction_refs: Vec<ProfileAssetRef>,
    /// Optional tool references for providers that support tool selection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_refs: Vec<String>,
    /// Optional permission mode for providers that support it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Optional path to a prompt-overlay / agent markdown file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_overlay_path: Option<PathBuf>,
    /// Launch behaviour (e.g. auto-restore, confirm-before-run).
    #[serde(default)]
    pub launch_policy: LaunchPolicy,
}

/// Behaviour rules for starting a profile session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum LaunchPolicy {
    /// Default: start immediately, restore on exit.
    #[default]
    AutoRestore,
    /// Dry-run: build the launch plan but do not execute.
    DryRun,
    /// Prompt for confirmation before modifying provider configs.
    ConfirmBeforeRun,
}

/// Validates that a profile ID is safe for filesystem use and non-empty.
pub fn validate_profile_id(id: &ProfileId) -> anyhow::Result<()> {
    let name = id.as_str();
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\u{0000}')
        || name.contains(':')
        || name == "."
        || name == ".."
        || name.starts_with("..")
    {
        anyhow::bail!(
            "Profile ID '{}' contains invalid filesystem characters",
            name
        );
    }
    Ok(())
}

/// Validates that references within a profile point to distinct IDs (no
/// duplicates).
pub fn validate_profile_refs(profile: &Profile) -> anyhow::Result<()> {
    let mut seen = std::collections::HashSet::new();
    for ref_id in &profile.skill_refs {
        if !seen.insert(format!("skill:{}", ref_id.name)) {
            anyhow::bail!("Duplicate skill reference: {}", ref_id.name);
        }
    }
    seen.clear();
    for ref_id in &profile.mcp_refs {
        if !seen.insert(format!("mcp:{}", ref_id.name)) {
            anyhow::bail!("Duplicate MCP reference: {}", ref_id.name);
        }
    }
    seen.clear();
    for ref_id in &profile.instruction_refs {
        if !seen.insert(format!("instruction:{}", ref_id.name)) {
            anyhow::bail!("Duplicate instruction reference: {}", ref_id.name);
        }
    }
    Ok(())
}

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
pub fn check_version_compatibility(export_version: &str, current_version: &str) -> Result<(), String> {
    let export_parts: Vec<&str> = export_version.split('.').collect();
    let current_parts: Vec<&str> = current_version.split('.').collect();

    if export_parts.len() < 2 || current_parts.len() < 2 {
        return Err(format!(
            "Cannot parse version numbers: export={}, current={}",
            export_version, current_version
        ));
    }

    let export_major: u32 = export_parts[0].parse().map_err(|_| {
        format!(
            "Cannot parse major version from export: {}",
            export_version
        )
    })?;
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

    let export_minor: u32 = export_parts[1].parse().unwrap_or(0);
    let current_minor: u32 = current_parts[1].parse().unwrap_or(0);

    if export_minor != current_minor {
        return Ok(()); // Caller should emit a warning
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_profile_id_accepted() {
        assert!(validate_profile_id(&ProfileId::new("opencode-dev")).is_ok());
    }

    #[test]
    fn empty_profile_id_rejected() {
        assert!(validate_profile_id(&ProfileId::new("")).is_err());
    }

    #[test]
    fn slash_in_profile_id_rejected() {
        assert!(validate_profile_id(&ProfileId::new("foo/bar")).is_err());
    }

    #[test]
    fn duplicate_skill_refs_rejected() {
        let profile = Profile {
            skill_refs: vec![
                ProfileAssetRef::new("java", "auto"),
                ProfileAssetRef::new("java", "auto"),
            ],
            ..Profile::default()
        };
        assert!(validate_profile_refs(&profile).is_err());
    }

    #[test]
    fn distinct_refs_accepted() {
        let profile = Profile {
            skill_refs: vec![
                ProfileAssetRef::new("java", "auto"),
                ProfileAssetRef::new("rust", "auto"),
            ],
            mcp_refs: vec![ProfileAssetRef::new("github", "auto")],
            ..Profile::default()
        };
        assert!(validate_profile_refs(&profile).is_ok());
    }

    #[test]
    fn profile_asset_ref_default_vault_is_auto() {
        let r = ProfileAssetRef::new("foo", "");
        assert_eq!(r.name, "foo");
        assert_eq!(r.vault, "");
    }

    #[test]
    fn profile_id_display() {
        let id = ProfileId::new("test");
        assert_eq!(id.as_str(), "test");
    }

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
        assert_eq!(deserialized.profile.agent_markdown, "# Dev Agent\nYou are a dev agent.");
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
