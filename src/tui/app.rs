use crate::domain::asset::{AssetKind, ProviderEntry, ScannedPackage, VaultEntry};
use crate::domain::config::{AssetKey, ConfigFile};
use crate::domain::scope::Scope;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabKind {
    Asset,
    Vault,
    Provider,
    Mcp,
    Analytics,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListMode {
    Normal,
    Searching,
    AttachVault,
    AttachVaultBranch,
    AttachVaultPath,
    AttachVaultName,
    ConfirmDetachVault,
    ConfirmClawHubInstall,
    ConfirmDeactivateLastProvider,
    /// MCP server registration modal sub-steps
    RegisterMcpStepName,
    RegisterMcpStepCommand,
    RegisterMcpStepArgs,
    RegisterMcpStepTransport,
    RegisterMcpStepDescription,
    ConfirmMcpTest,
    SelectProviderRoot {
        provider_id: String,
        options: Vec<(String, String)>,
        selected: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProgressStatus {
    Starting,
    Running(u8),
}

#[derive(Clone, Debug)]
pub struct Progress {
    pub name: String,
    pub status: ProgressStatus,
}

pub static NEXT_TASK_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

pub struct AppState {
    pub active_tab: usize,
    pub search_query: String,
    pub selected_index: usize,
    pub list_mode: ListMode,
    pub status_line: String,
    pub tab_names: Vec<String>,
    pub tab_live: Vec<bool>,
    pub packages: HashMap<usize, Vec<ScannedPackage>>,
    pub active_scope: Scope,
    pub checked_items: HashSet<AssetKey>,
    pub prompt_buffer: String,
    pub configs: HashMap<Scope, ConfigFile>,
    pub tab_kinds: Vec<TabKind>,
    pub vault_entries: Vec<VaultEntry>,
    pub provider_entries: Vec<ProviderEntry>,
    pub active_tasks: HashMap<usize, Progress>,
    pub latest_task_id: Option<usize>,
    pub pending_detach_vault: Option<String>,
    pub pending_vault_id: String,
    pub pending_vault_url: String,
    pub pending_vault_repo: String,
    pub pending_vault_ref: String,
    pub pending_vault_path: String,
    pub pending_vault_local_path: String,
    pub pending_deactivate_provider_id: String,
    pub esc_pressed_once: bool,
    pub remote_packages: Vec<ScannedPackage>,
    pub clawhub_search_task_id: Option<usize>,
    pub scroll_offset: usize,
    pub scroll_tick: u8,
    pub analytics_config: crate::domain::telemetry::AnalyticsConfig,
    pub mcp_state: crate::tui::widgets::mcp::McpState,
    // MCP registration pending fields (for modal)
    pub pending_mcp_name: String,
    pub pending_mcp_command: String,
    pub pending_mcp_args: String,
    pub pending_mcp_transport: String,
    pub pending_mcp_description: String,
}

impl AppState {
    pub fn new(
        tab_names: Vec<String>,
        tab_live: Vec<bool>,
        packages: HashMap<usize, Vec<ScannedPackage>>,
    ) -> Self {
        Self {
            active_tab: 0,
            search_query: String::new(),
            selected_index: 0,
            list_mode: ListMode::Normal,
            status_line: String::new(),
            tab_names,
            tab_live,
            packages,
            prompt_buffer: String::new(),
            active_scope: Scope::Workspace,
            checked_items: HashSet::new(),
            configs: HashMap::new(),
            tab_kinds: Vec::new(),
            vault_entries: Vec::new(),
            provider_entries: Vec::new(),
            active_tasks: HashMap::new(),
            latest_task_id: None,
            pending_detach_vault: None,
            pending_vault_id: String::new(),
            pending_vault_url: String::new(),
            pending_vault_repo: String::new(),
            pending_vault_ref: String::new(),
            pending_vault_path: String::new(),
            pending_vault_local_path: String::new(),
            pending_deactivate_provider_id: String::new(),
            esc_pressed_once: false,
            remote_packages: Vec::new(),
            clawhub_search_task_id: None,
            scroll_offset: 0,
            scroll_tick: 0,
            analytics_config: crate::domain::telemetry::AnalyticsConfig::default(),
            mcp_state: crate::tui::widgets::mcp::McpState::default(),
            pending_mcp_name: String::new(),
            pending_mcp_command: String::new(),
            pending_mcp_args: String::new(),
            pending_mcp_transport: String::new(),
            pending_mcp_description: String::new(),
        }
    }

    pub fn active_packages(&self) -> &[ScannedPackage] {
        self.packages
            .get(&self.active_tab)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn filtered_packages(&self) -> Vec<&ScannedPackage> {
        let pkgs = self.active_packages();
        let q = self.search_query.to_lowercase();
        let mut result: Vec<&ScannedPackage> = if q.is_empty() {
            pkgs.iter().collect()
        } else {
            pkgs.iter()
                .filter(|p| p.identity.name.to_lowercase().contains(&q))
                .collect()
        };
        // Merge remote ClawHub results, deduplicating by name (local wins)
        if !self.search_query.is_empty() {
            let local_names: std::collections::HashSet<&str> =
                result.iter().map(|p| p.identity.name.as_str()).collect();
            for remote_pkg in &self.remote_packages {
                if !local_names.contains(remote_pkg.identity.name.as_str()) {
                    result.push(remote_pkg);
                }
            }
        }
        result
    }

    pub fn list_length(&self) -> usize {
        match self.tab_kinds.get(self.active_tab) {
            Some(TabKind::Vault) => self.vault_entries.len(),
            Some(TabKind::Provider) => self.provider_entries.len(),
            Some(TabKind::Mcp) => self.mcp_state.servers_list().len(),
            _ => self.filtered_packages().len(),
        }
    }

    pub fn is_active_tab_live(&self) -> bool {
        match self.tab_kinds.get(self.active_tab) {
            Some(TabKind::Vault) | Some(TabKind::Provider) => true,
            _ => self.tab_live.get(self.active_tab).copied().unwrap_or(false),
        }
    }

    pub fn toggle_scope(&mut self) {
        self.active_scope = match self.active_scope {
            Scope::Global => Scope::Workspace,
            Scope::Workspace => Scope::Global,
        };
    }

    pub fn active_config(&self) -> &ConfigFile {
        static EMPTY: std::sync::OnceLock<ConfigFile> = std::sync::OnceLock::new();
        self.configs
            .get(&self.active_scope)
            .unwrap_or_else(|| EMPTY.get_or_init(ConfigFile::default))
    }

    pub fn is_installed(&self, vault_id: &str, name: &str, kind: &AssetKind) -> bool {
        let config = self.active_config();
        match kind {
            AssetKind::Skill => config.is_skill_installed(vault_id, name),
            AssetKind::Instruction => config.is_instruction_installed(vault_id, name),
            AssetKind::McpServer => false,
        }
    }

    pub fn active_scope_has_provider(&self) -> bool {
        !self.active_config().providers.is_empty()
    }

    pub fn scope_label(&self) -> &'static str {
        match self.active_scope {
            Scope::Global => "[Tab] GLOBAL",
            Scope::Workspace => "[Tab] WORKSPACE",
        }
    }

    pub fn progress_summary(&self) -> Option<String> {
        let total = self.active_tasks.len();
        if total == 0 {
            return None;
        }

        let latest = self
            .latest_task_id
            .and_then(|id| self.active_tasks.get(&id))
            .or_else(|| self.active_tasks.values().next())?;

        let prefix = &latest.name;
        match &latest.status {
            ProgressStatus::Starting => Some(format!("{} ... ({} tasks)", prefix, total)),
            ProgressStatus::Running(pct) => {
                Some(format!("{} ... {}% ({} tasks)", prefix, pct, total))
            }
        }
    }

    pub fn is_attach_vault_mode(&self) -> bool {
        matches!(
            self.list_mode,
            ListMode::AttachVault
                | ListMode::AttachVaultBranch
                | ListMode::AttachVaultPath
                | ListMode::AttachVaultName
        )
    }

    pub fn is_register_mcp_mode(&self) -> bool {
        matches!(
            self.list_mode,
            ListMode::RegisterMcpStepName
                | ListMode::RegisterMcpStepCommand
                | ListMode::RegisterMcpStepArgs
                | ListMode::RegisterMcpStepTransport
                | ListMode::RegisterMcpStepDescription
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::asset::{AssetKind, ScannedPackage};
    use crate::domain::identity::AssetIdentity;
    use crate::domain::scope::Scope;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_pkg(name: &str) -> ScannedPackage {
        ScannedPackage {
            identity: AssetIdentity::new(name, None, "0000000000"),
            path: PathBuf::from("/skills").join(name),
            vault_id: "workspace".to_string(),
            kind: AssetKind::Skill,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
        }
    }

    fn state_with_skills(pkgs: Vec<ScannedPackage>) -> AppState {
        let mut packages = HashMap::new();
        packages.insert(0usize, pkgs);
        AppState::new(
            vec!["Skills".to_string(), "Instructions".to_string()],
            vec![true, false],
            packages,
        )
    }

    #[test]
    fn active_packages_returns_current_tab() {
        let state = state_with_skills(vec![make_pkg("alpha"), make_pkg("beta")]);
        assert_eq!(state.active_packages().len(), 2);
    }

    #[test]
    fn filtered_packages_empty_query_returns_all() {
        let state = state_with_skills(vec![make_pkg("alpha"), make_pkg("beta")]);
        assert_eq!(state.filtered_packages().len(), 2);
    }

    #[test]
    fn filtered_packages_filters_by_name() {
        let state = state_with_skills(vec![make_pkg("alpha-skill"), make_pkg("beta-tool")]);
        let mut s = state;
        s.search_query = "alpha".to_string();
        let filtered = s.filtered_packages();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].identity.name, "alpha-skill");
    }

    #[test]
    fn filtered_packages_case_insensitive() {
        let state = state_with_skills(vec![make_pkg("MySkill")]);
        let mut s = state;
        s.search_query = "myskill".to_string();
        assert_eq!(s.filtered_packages().len(), 1);
    }

    #[test]
    fn filtered_packages_merges_remote_results() {
        let mut state = state_with_skills(vec![make_pkg("local-skill")]);
        state.search_query = "skill".to_string();
        let remote_pkg = ScannedPackage {
            identity: AssetIdentity::new("remote-skill", None, "----------"),
            path: PathBuf::new(),
            vault_id: "clawhub".to_string(),
            kind: AssetKind::Skill,
            is_remote: true,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
        };
        state.remote_packages = vec![remote_pkg];
        let filtered = state.filtered_packages();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filtered_packages_deduplicates_remote() {
        let mut state = state_with_skills(vec![make_pkg("same-skill")]);
        state.search_query = "same".to_string();
        let remote_pkg = ScannedPackage {
            identity: AssetIdentity::new("same-skill", None, "----------"),
            path: PathBuf::new(),
            vault_id: "clawhub".to_string(),
            kind: AssetKind::Skill,
            is_remote: true,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
        };
        state.remote_packages = vec![remote_pkg];
        let filtered = state.filtered_packages();
        assert_eq!(filtered.len(), 1);
        assert!(!filtered[0].is_remote);
    }

    #[test]
    fn default_active_tab_is_zero() {
        let state = state_with_skills(vec![]);
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn scope_starts_workspace() {
        let state = state_with_skills(vec![]);
        assert_eq!(state.active_scope, Scope::Workspace);
    }

    #[test]
    fn toggle_scope_switches_to_global() {
        let mut state = state_with_skills(vec![]);
        state.toggle_scope();
        assert_eq!(state.active_scope, Scope::Global);
    }

    #[test]
    fn toggle_scope_switches_back_to_workspace() {
        let mut state = state_with_skills(vec![]);
        state.toggle_scope();
        state.toggle_scope();
        assert_eq!(state.active_scope, Scope::Workspace);
    }

    #[test]
    fn is_installed_false_for_empty_config() {
        let state = state_with_skills(vec![]);
        assert!(!state.is_installed("workspace", "any-skill", &AssetKind::Skill));
    }

    #[test]
    fn tab_kind_vaults_and_providers() {
        let mut state = AppState::new(
            vec![
                "Skills".into(),
                "MCP Servers".into(),
                "Instructions".into(),
                "Providers".into(),
                "Vaults".into(),
            ],
            vec![true, true, true, true, true],
            HashMap::new(),
        );
        state.tab_kinds = vec![
            TabKind::Asset,
            TabKind::Mcp,
            TabKind::Asset,
            TabKind::Provider,
            TabKind::Vault,
        ];
        assert_eq!(state.tab_kinds[0], TabKind::Asset); // Skills
        assert_eq!(state.tab_kinds[1], TabKind::Mcp); // MCP
        assert_eq!(state.tab_kinds[3], TabKind::Provider); // Providers
        assert_eq!(state.tab_kinds[4], TabKind::Vault); // Vault
    }
}
