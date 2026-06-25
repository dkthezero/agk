/// Payload for [`CoreCommand::CreateProfile`].
#[derive(Debug, Clone, PartialEq)]
pub struct CreateProfileInput {
    pub id: crate::domain::profile::ProfileId,
    pub provider_id: crate::domain::profile::ProviderId,
    pub skill_refs: Vec<crate::domain::profile::ProfileAssetRef>,
    pub mcp_refs: Vec<crate::domain::profile::ProfileAssetRef>,
    pub instruction_refs: Vec<crate::domain::profile::ProfileAssetRef>,
    pub description: String,
    pub scope: crate::domain::scope::Scope,
    /// Free-form model name (e.g. `claude-sonnet-4-5`, `sonnet`). Used by the
    /// `claude-code` provider branch when rendering the agent markdown.
    pub model: Option<String>,
    /// Optional LLM provider id the downstream exec should use.
    pub llm_provider_id: Option<String>,
    /// MCP server definitions resolved at create-time. Embedded into the
    /// generated agent markdown for the `claude-code` provider branch.
    pub agent_mcp_servers: Vec<crate::domain::agent_markdown::AgentMcpServer>,
    /// Optional tool references (e.g. `Read`, `Grep`). Embedded into the
    /// generated agent markdown for the `claude-code` provider branch.
    pub tool_refs: Vec<String>,
    /// Optional permission mode (e.g. `acceptEdits`). Embedded into the
    /// generated agent markdown for the `claude-code` provider branch.
    pub permission_mode: Option<String>,
    /// When true, the use case emits a preview (LaunchPlan-style) instead of
    /// shelling out to the provider CLI, writing agent markdown, or persisting
    /// the new profile to config. Honours the `--dry-run` contract: no fs
    /// side effects, no provider invocation, no config mutation.
    pub dry_run: bool,
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
            model: None,
            llm_provider_id: None,
            agent_mcp_servers: Vec::new(),
            tool_refs: Vec::new(),
            permission_mode: None,
            dry_run: false,
        }
    }
}

/// Patch for [`CoreCommand::UpdateProfile`].  Only non-None fields are mutated.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UpdateProfilePatch {
    pub provider_id: Option<crate::domain::profile::ProviderId>,
    pub skill_refs: Option<Vec<crate::domain::profile::ProfileAssetRef>>,
    pub mcp_refs: Option<Vec<crate::domain::profile::ProfileAssetRef>>,
    pub instruction_refs: Option<Vec<crate::domain::profile::ProfileAssetRef>>,
    pub tool_refs: Option<Vec<String>>,
    pub permission_mode: Option<Option<String>>,
    pub prompt_overlay_path: Option<Option<std::path::PathBuf>>,
    pub launch_policy: Option<crate::domain::profile::LaunchPolicy>,
}
