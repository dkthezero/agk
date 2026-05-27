use serde::{Deserialize, Serialize};
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
}

/// Typed wrapper for MCP server identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct McpServerId(pub String);

impl McpServerId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Typed wrapper for instruction identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstructionId(pub String);

impl InstructionId {
    #[allow(dead_code)]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
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
    /// IDs of skills to activate when the profile runs.
    #[serde(default)]
    pub skill_refs: Vec<SkillId>,
    /// IDs of MCP servers to inject when the profile runs.
    #[serde(default)]
    pub mcp_refs: Vec<McpServerId>,
    /// IDs of instructions to overlay.
    #[serde(default)]
    pub instruction_refs: Vec<InstructionId>,
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
        if !seen.insert(format!("skill:{}", ref_id.0)) {
            anyhow::bail!("Duplicate skill reference: {}", ref_id.0);
        }
    }
    seen.clear();
    for ref_id in &profile.mcp_refs {
        if !seen.insert(format!("mcp:{}", ref_id.0)) {
            anyhow::bail!("Duplicate MCP reference: {}", ref_id.0);
        }
    }
    seen.clear();
    for ref_id in &profile.instruction_refs {
        if !seen.insert(format!("instruction:{}", ref_id.0)) {
            anyhow::bail!("Duplicate instruction reference: {}", ref_id.0);
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
            skill_refs: vec![SkillId::new("java"), SkillId::new("java")],
            ..Profile::default()
        };
        assert!(validate_profile_refs(&profile).is_err());
    }

    #[test]
    fn distinct_refs_accepted() {
        let profile = Profile {
            skill_refs: vec![SkillId::new("java"), SkillId::new("rust")],
            mcp_refs: vec![McpServerId::new("github")],
            ..Profile::default()
        };
        assert!(validate_profile_refs(&profile).is_ok());
    }

    #[test]
    fn profile_id_display() {
        let id = ProfileId::new("test");
        assert_eq!(id.as_str(), "test");
    }
}
