/// Central input contract shared by the TUI and CLI.
///
/// Both interface adapters translate user actions into `CoreCommand` variants.
/// The [`crate::app::core::AgkCore`] façade receives these commands and routes
/// them to the appropriate use case.  This guarantees that headless and
/// interactive behaviour can never diverge.
/// NOTE: Variants are wired incrementally as use-cases migrate into `core.rs`.
/// Dead-code warnings are suppressed because every variant has a planned home.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CoreCommand {
    // -----------------------------------------------------------------------
    // Profile commands
    // -----------------------------------------------------------------------
    ListProfiles {
        scope: Option<crate::domain::scope::Scope>,
    },
    CreateProfile {
        input: CreateProfileInput,
    },
    UpdateProfile {
        id: crate::domain::profile::ProfileId,
        patch: UpdateProfilePatch,
    },
    DeleteProfile {
        id: crate::domain::profile::ProfileId,
        scope: crate::domain::scope::Scope,
    },
    AttachSkillToProfile {
        profile_id: crate::domain::profile::ProfileId,
        skill_id: crate::domain::profile::SkillId,
    },
    DetachSkillFromProfile {
        profile_id: crate::domain::profile::ProfileId,
        skill_id: crate::domain::profile::SkillId,
    },
    AttachMcpToProfile {
        profile_id: crate::domain::profile::ProfileId,
        mcp_id: crate::domain::profile::McpServerId,
    },
    DetachMcpFromProfile {
        profile_id: crate::domain::profile::ProfileId,
        mcp_id: crate::domain::profile::McpServerId,
    },
    ValidateProfile {
        id: crate::domain::profile::ProfileId,
        scope: crate::domain::scope::Scope,
    },
    StartProfile {
        id: crate::domain::profile::ProfileId,
        scope: crate::domain::scope::Scope,
        dry_run: bool,
    },

    // -----------------------------------------------------------------------
    // Vault commands
    // -----------------------------------------------------------------------
    AttachVault {
        input: AttachVaultInput,
    },
    DetachVault {
        vault_id: String,
        scope: crate::domain::scope::Scope,
    },
    RefreshVault {
        vault_id: String,
    },

    // -----------------------------------------------------------------------
    // Provider commands
    // -----------------------------------------------------------------------
    ActivateProvider {
        id: String,
        scope: crate::domain::scope::Scope,
    },
    DeactivateProvider {
        id: String,
        scope: crate::domain::scope::Scope,
    },

    // -----------------------------------------------------------------------
    // MCP commands
    // -----------------------------------------------------------------------
    RegisterMcp {
        input: RegisterMcpInput,
    },
    EnableMcp {
        name: String,
        provider_id: String,
        scope: crate::domain::scope::Scope,
    },
    DisableMcp {
        name: String,
        provider_id: String,
        scope: crate::domain::scope::Scope,
    },

    // -----------------------------------------------------------------------
    // Asset commands
    // -----------------------------------------------------------------------
    InstallAsset {
        identity: String,
        scope: crate::domain::scope::Scope,
        provider_filter: Option<String>,
        include_evals: bool,
        dry_run: bool,
    },
    RemoveAsset {
        identity: String,
        scope: crate::domain::scope::Scope,
        provider_filter: Option<String>,
    },
    UpdateAsset {
        identity: String,
        scope: crate::domain::scope::Scope,
        provider_filter: Option<String>,
    },
    SyncAssets {
        scope: crate::domain::scope::Scope,
        dry_run: bool,
    },
    SearchRemoteVault {
        vault_id: String,
        query: String,
    },

    // -----------------------------------------------------------------------
    // Context commands
    // -----------------------------------------------------------------------
    SwitchContext {
        id: crate::domain::context::ContextId,
        dry_run: bool,
    },
    ListContexts,

    // -----------------------------------------------------------------------
    // Apply commands
    // -----------------------------------------------------------------------
    ApplyConfig {
        input: ApplyConfigInput,
        scope: crate::domain::scope::Scope,
        environment: Option<crate::domain::context::Environment>,
        context: Option<crate::domain::context::ContextId>,
        dry_run: bool,
    },

    // -----------------------------------------------------------------------
    // Workspace commands
    // -----------------------------------------------------------------------
    LoadWorkspaceSnapshot {
        scope: crate::domain::scope::Scope,
    },
}

// ---------------------------------------------------------------------------
// Input structs
// ---------------------------------------------------------------------------

/// A vault to attach as part of `apply`.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyVault {
    pub id: String,
    pub config: crate::domain::config::VaultConfig,
}

/// Payload for [`CoreCommand::ApplyConfig`].
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyConfigInput {
    pub source_url: String,
    pub vaults: Vec<ApplyVault>,
    pub providers: Vec<String>,
    pub profiles: Vec<crate::domain::config::Profile>,
}

impl ApplyConfigInput {
    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            source_url: url.into(),
            vaults: Vec::new(),
            providers: Vec::new(),
            profiles: Vec::new(),
        }
    }

    #[allow(dead_code)] // builder methods used in tests / apply_config use-case
    pub fn with_vault(
        mut self,
        id: impl Into<String>,
        config: crate::domain::config::VaultConfig,
    ) -> Self {
        self.vaults.push(ApplyVault {
            id: id.into(),
            config,
        });
        self
    }

    #[allow(dead_code)]
    pub fn with_provider(mut self, id: impl Into<String>) -> Self {
        self.providers.push(id.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_profile(mut self, profile: crate::domain::config::Profile) -> Self {
        self.profiles.push(profile);
        self
    }
}

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

/// Payload for [`CoreCommand::AttachVault`].
#[derive(Debug, Clone, PartialEq)]
pub struct AttachVaultInput {
    pub vault_id: String,
    pub config: crate::domain::config::VaultConfig,
    pub scope: crate::domain::scope::Scope,
}

/// Payload for [`CoreCommand::RegisterMcp`].
#[derive(Debug, Clone, PartialEq)]
pub struct RegisterMcpInput {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub transport: crate::domain::mcp::McpTransport,
    pub description: Option<String>,
    pub test_after: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_profile_defaults_empty() {
        let input = CreateProfileInput::new("", "", crate::domain::scope::Scope::Workspace);
        assert_eq!(input.id.as_str(), "");
        assert!(input.skill_refs.is_empty());
        assert!(input.mcp_refs.is_empty());
    }

    #[test]
    fn update_profile_patch_all_none() {
        let patch = UpdateProfilePatch::default();
        assert!(patch.provider_id.is_none());
        assert!(patch.skill_refs.is_none());
    }
}
