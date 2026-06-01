use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use super::profile_export::*;

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
}
