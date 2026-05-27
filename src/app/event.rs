/// Events emitted by the application core back to the TUI and CLI presenters.
///
/// These are **facts** that happened as a result of executing a [`crate::app::command::CoreCommand`].
/// Both TUI and CLI observe the same events, but render them differently (UI state
/// updates vs stdout/JSON).
/// NOTE: Events are emitted incrementally as use-cases are wired into core.rs.
#[derive(Debug, Clone, PartialEq)]
pub enum CoreEvent {
    // -----------------------------------------------------------------------
    // Workspace / lifecycle
    // -----------------------------------------------------------------------
    WorkspaceLoaded(WorkspaceSnapshot),

    // -----------------------------------------------------------------------
    // Profiles
    // -----------------------------------------------------------------------
    ProfileCreated(crate::domain::profile::ProfileId),
    ProfileUpdated(crate::domain::profile::ProfileId),
    ProfileDeleted(crate::domain::profile::ProfileId),
    ProfileValidated {
        id: crate::domain::profile::ProfileId,
        valid: bool,
        message: String,
    },
    ProfileLaunchPlan {
        id: crate::domain::profile::ProfileId,
        plan: LaunchPlan,
    },
    ProfileSessionStarted {
        id: crate::domain::profile::ProfileId,
        session_key: String,
    },
    ProfileSessionFinished {
        id: crate::domain::profile::ProfileId,
        exit_status: Option<i32>,
    },

    // -----------------------------------------------------------------------
    // Vaults
    // -----------------------------------------------------------------------
    VaultAttached(String),
    VaultDetached(String),
    VaultRefreshed(String),

    // -----------------------------------------------------------------------
    // Providers
    // -----------------------------------------------------------------------
    ProviderActivated(String),
    ProviderDeactivated(String),

    // -----------------------------------------------------------------------
    // MCP
    // -----------------------------------------------------------------------
    McpRegistered(String),
    McpEnabled {
        name: String,
        provider_id: String,
    },
    McpDisabled {
        name: String,
        provider_id: String,
    },

    // -----------------------------------------------------------------------
    // Assets
    // -----------------------------------------------------------------------
    AssetInstalled {
        identity: String,
        providers: Vec<String>,
    },
    AssetRemoved {
        identity: String,
    },
    AssetUpdated {
        identity: String,
    },
    SyncComplete {
        updated: Vec<String>,
        skipped: Vec<String>,
        errors: Vec<String>,
    },
    RemoteVaultSearchResults {
        vault_id: String,
        packages: Vec<crate::domain::asset::ScannedPackage>,
    },

    // -----------------------------------------------------------------------
    // Tasks / progress
    // -----------------------------------------------------------------------
    TaskStarted {
        id: usize,
        name: String,
    },
    TaskProgress {
        id: usize,
        percent: u8,
    },
    TaskCompleted {
        id: usize,
        message: String,
    },
    TaskFailed {
        id: usize,
        error: String,
    },

    ValidationReport {
        passed: bool,
        message: String,
    },

    // -----------------------------------------------------------------------
    // Errors
    // -----------------------------------------------------------------------
    Error(String),
}

/// A concrete, serialisable plan for what a profile session will do.
/// Returned when `dry_run = true` so callers can inspect side-effects before
/// committing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LaunchPlan {
    pub profile_id: crate::domain::profile::ProfileId,
    pub provider_id: crate::domain::profile::ProviderId,
    pub skills: Vec<crate::domain::profile::SkillId>,
    pub mcps: Vec<crate::domain::profile::McpServerId>,
    pub files_to_write: Vec<std::path::PathBuf>,
    /// Whether provider config restoration is required after the session.
    pub restore_required: bool,

    // --- Provider-specific concrete plan details ---
    /// Path to the base agent markdown that will be copied for this session.
    pub agent_markdown_source: std::path::PathBuf,
    /// Patched provider configuration (e.g. opencode.json) to be written.
    pub patched_provider_config: Option<serde_json::Value>,
    /// Original provider configuration bytes for lossless restoration.
    pub original_provider_config_bytes: Option<Vec<u8>>,
}

/// Snapshot of the workspace configuration and scan results.
/// Treated as an immutable read model — presenters derive TUI/CLI view state
/// from this without mutating it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorkspaceSnapshot {
    pub scope: crate::domain::scope::Scope,
    pub vaults: Vec<crate::app::snapshot::VaultEntry>,
    pub providers: Vec<crate::app::snapshot::ProviderEntry>,
    pub profiles: Vec<crate::app::snapshot::ProfileEntry>,
    pub mcp_servers: Vec<McpView>,
    pub packages_by_tab: Vec<Vec<crate::domain::asset::ScannedPackage>>,
    pub active_tasks: Vec<TaskView>,
}

/// Lightweight MCP view for the snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct McpView {
    pub id: String,
    pub command: String,
    pub transport: String,
    pub enabled: bool,
}

/// Task representation for the snapshot (mirrors tui::app::Progress but lives
/// in the app layer so CLI can render it too).
#[derive(Debug, Clone, PartialEq)]
pub struct TaskView {
    pub id: usize,
    pub name: String,
    pub percent: Option<u8>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Running,
    Completed,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_snapshot_default_empty() {
        let snap = WorkspaceSnapshot::default();
        assert!(snap.vaults.is_empty());
        assert!(snap.packages_by_tab.is_empty());
    }

    #[test]
    fn launch_plan_default_empty() {
        let plan = LaunchPlan::default();
        assert!(plan.files_to_write.is_empty());
        assert!(!plan.restore_required);
    }
}
