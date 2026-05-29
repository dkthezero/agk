/// Payload for [`CoreCommand::CreateProfile`].
#[derive(Debug, Clone, PartialEq)]
pub struct CreateProfileInput {
    pub id: crate::domain::profile::ProfileId,
    pub provider_id: crate::domain::profile::ProviderId,
    pub skill_refs: Vec<crate::domain::profile::SkillId>,
    pub mcp_refs: Vec<crate::domain::profile::McpServerId>,
    pub instruction_refs: Vec<crate::domain::profile::InstructionId>,
    pub description: String,
    pub scope: crate::domain::scope::Scope,
}

impl CreateProfileInput {
    pub fn new(
        id: impl Into<crate::domain::profile::ProfileId>,
        provider_id: impl Into<crate::domain::profile::ProviderId>,
        scope: crate::domain::scope::Scope,
    ) -> Self {
        Self {
            id: id.into(),
            provider_id: provider_id.into(),
            skill_refs: Vec::new(),
            mcp_refs: Vec::new(),
            instruction_refs: Vec::new(),
            description: String::new(),
            scope,
        }
    }
}

/// Patch for [`CoreCommand::UpdateProfile`].  Only non-None fields are mutated.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UpdateProfilePatch {
    pub provider_id: Option<crate::domain::profile::ProviderId>,
    pub skill_refs: Option<Vec<crate::domain::profile::SkillId>>,
    pub mcp_refs: Option<Vec<crate::domain::profile::McpServerId>>,
    pub instruction_refs: Option<Vec<crate::domain::profile::InstructionId>>,
    pub prompt_overlay_path: Option<Option<std::path::PathBuf>>,
    pub launch_policy: Option<crate::domain::profile::LaunchPolicy>,
}
