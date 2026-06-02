use crate::domain::config::vault_section::AssetSource;
use crate::tui::app::AppState;
use crate::tui::list_mode::ListMode;
use crate::tui::progress::ProgressStatus;

impl AppState {
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

    pub fn is_profile_wizard_mode(&self) -> bool {
        matches!(self.list_mode, ListMode::ProfileWizard)
    }

    /// Compute a team status summary for the active scope config.
    ///
    /// Returns `Some((installed, required, personal))` if any vault section
    /// has a team source; `None` if no team-mandated assets exist.
    pub fn team_status(&self) -> Option<(usize, usize, usize)> {
        let config = self.active_config();
        let mut team_installed = 0usize;
        let mut team_required = 0usize;
        let mut personal = 0usize;

        for section in config.vault_defs.values() {
            for bucket in [
                section.skills.as_ref(),
                section.instructions.as_ref(),
                section.mcps.as_ref(),
                section.profiles.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                let count = bucket.items.len();
                match bucket.source.as_ref().unwrap_or(&AssetSource::Personal) {
                    AssetSource::Team => {
                        team_required += count;
                        team_installed += count; // items in the bucket are installed
                    }
                    AssetSource::Personal => {
                        personal += count;
                    }
                }
            }
        }

        if team_required > 0 {
            Some((team_installed, team_required, personal))
        } else {
            None
        }
    }
}
