/// Central input contract shared by the TUI and CLI.
///
/// Both interface adapters translate user actions into `CoreCommand` variants.
/// The [`crate::app::core::AgkCore`] façade receives these commands and routes
/// them to the appropriate use case.  This guarantees that headless and
/// interactive behaviour can never diverge.
///
/// Feature-specific input structs are defined in their respective
/// `app/features/<f>/command.rs` files and re-exported here for convenience.
#[derive(Debug, Clone, PartialEq)]
pub enum CoreCommand {
    // -----------------------------------------------------------------------
    // Profile commands
    // -----------------------------------------------------------------------
    ListProfiles {
        scope: Option<crate::domain::scope::Scope>,
    },
    CreateProfile {
        input: crate::app::features::profile::command::CreateProfileInput,
    },
    UpdateProfile {
        id: crate::domain::profile::ProfileId,
        patch: crate::app::features::profile::command::UpdateProfilePatch,
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
        input: crate::app::features::vault::command::AttachVaultInput,
    },
    DetachVault {
        vault_id: String,
        scope: crate::domain::scope::Scope,
    },
    AttachBareVault {
        vault_id: String,
        scope: crate::domain::scope::Scope,
    },
    RefreshVault {
        vault_id: String,
    },
    RefreshAllVaults,

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
        input: crate::app::features::mcp::command::RegisterMcpInput,
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
    ListMcp,
    TestMcp {
        name: String,
    },
    ToggleMcp {
        name: String,
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
    ValidateAssets {
        scope: crate::domain::scope::Scope,
    },
    PackAsset {
        identity: String,
        target: crate::domain::asset::PackTarget,
        stdout: bool,
        scope: crate::domain::scope::Scope,
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
        input: crate::app::features::apply::command::ApplyConfigInput,
        scope: crate::domain::scope::Scope,
        environment: Option<crate::domain::context::Environment>,
        context: Option<crate::domain::context::ContextId>,
        dry_run: bool,
    },

    // -----------------------------------------------------------------------
    // Telemetry commands
    // -----------------------------------------------------------------------
    EnableTelemetry,
    DisableTelemetry,
    TelemetryStatus,
    ExportTelemetry {
        format: crate::domain::telemetry::TelemetryExportFormat,
        output_path: Option<String>,
    },

    // -----------------------------------------------------------------------
    // Debug / observability commands
    // -----------------------------------------------------------------------
    DebugListTasks,
    DebugDetectHangs,
    DebugDumpTrace,

    // -----------------------------------------------------------------------
    // Workspace commands
    // -----------------------------------------------------------------------
    CleanWorkspace {
        global: bool,
    },
    LoadWorkspaceSnapshot {
        scope: crate::domain::scope::Scope,
    },
}

#[cfg(test)]
mod tests {
    use crate::app::features::profile::command::CreateProfileInput;
    use crate::app::features::profile::command::UpdateProfilePatch;

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
