use crate::app::features::common::parse_identity_from_item;
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
    /// Returns `Some((installed, required, personal))` if a team config is
    /// loaded; `None` if no team is active.
    ///
    /// `required` comes from the team config requirements list.
    /// `installed` counts how many of those requirements are present in
    /// the installed config (matching by identity in the correct vault
    /// and kind bucket).
    /// `personal` counts items in non-team buckets.
    pub fn team_status(&self) -> Option<(usize, usize, usize)> {
        let team_config = self.team_config.as_ref()?;
        let config = self.active_config();

        let required = team_config.requirements.len();
        let mut installed = 0usize;

        for req in &team_config.requirements {
            if let Some(section) = config.vault_defs.get(&req.vault) {
                let bucket = match req.kind {
                    crate::domain::asset::AssetKind::Skill => &section.skills,
                    crate::domain::asset::AssetKind::Instruction => &section.instructions,
                    crate::domain::asset::AssetKind::McpServer => &section.mcps,
                    crate::domain::asset::AssetKind::Profile => &section.profiles,
                };
                let found = bucket
                    .as_ref()
                    .map(|b| {
                        b.items.iter().any(|item| {
                            parse_identity_from_item(item).as_deref() == Some(req.identity.as_str())
                        })
                    })
                    .unwrap_or(false);
                if found {
                    installed += 1;
                }
            }
        }

        let mut personal = 0usize;
        for (vault_id, section) in &config.vault_defs {
            let is_team_vault = team_config.vaults.iter().any(|v| v.identity == *vault_id);
            if is_team_vault {
                continue;
            }
            if let Some(ref bucket) = section.skills {
                personal += bucket.items.len();
            }
            if let Some(ref bucket) = section.instructions {
                personal += bucket.items.len();
            }
            if let Some(ref bucket) = section.mcps {
                personal += bucket.items.len();
            }
            if let Some(ref bucket) = section.profiles {
                personal += bucket.items.len();
            }
        }

        if required > 0 {
            Some((installed, required, personal))
        } else {
            None
        }
    }
}
