use crate::tui::app::AppState;

/// Apply a [`CoreEvent`] to the TUI [`AppState`].
///
/// Task events mirror the existing [`AppEvent`] task arms.
/// Other events update the status line; detailed controller-specific state
/// mutations happen in Commit 9.
pub fn apply_core_event(state: &mut AppState, event: &crate::app::event::CoreEvent) {
    use crate::app::event::CoreEvent;
    match event {
        CoreEvent::TaskStarted { id, name } => {
            state.latest_task_id = Some(*id);
            state.active_tasks.insert(
                *id,
                crate::tui::progress::Progress {
                    name: name.clone(),
                    status: crate::tui::progress::ProgressStatus::Starting,
                },
            );
            // Track asset operations so the list can show a spinner.
            if *id == 0 {
                if let Some(rest) = name.strip_prefix("Installing '") {
                    if let Some(identity) = rest.strip_suffix("'") {
                        state.installing_names.insert(identity.to_string());
                    }
                }
            }
        }
        CoreEvent::TaskProgress { id, percent } => {
            if let Some(task) = state.active_tasks.get_mut(id) {
                task.status = crate::tui::progress::ProgressStatus::Running(*percent);
            }
        }
        CoreEvent::TaskCompleted { id, message } => {
            state.active_tasks.remove(id);
            state.status_line = message.clone();
        }
        CoreEvent::TaskFailed { id, error } => {
            state.active_tasks.remove(id);
            if *id == 0 {
                state.installing_names.clear();
            }
            state.status_line = format!("Error: {}", error);
        }
        CoreEvent::ProfileCreated(id) => {
            state.status_line = format!("Profile '{}' created", id.as_str());
        }
        CoreEvent::ProfileUpdated(id) => {
            state.status_line = format!("Profile '{}' updated", id.as_str());
        }
        CoreEvent::ProfileDeleted(id) => {
            state.status_line = format!("Profile '{}' deleted", id.as_str());
        }
        CoreEvent::ProfileValidated { id, valid, message } => {
            state.status_line = format!(
                "Profile '{}' validation: {} ({})",
                id.as_str(),
                if *valid { "pass" } else { "fail" },
                message
            );
        }
        CoreEvent::ProfileLaunchPlan { id, .. } => {
            state.status_line = format!("Launch plan ready for '{}'", id.as_str());
        }
        CoreEvent::ProfileSessionStarted { id, .. } => {
            state.status_line = format!("Session started for '{}'", id.as_str());
        }
        CoreEvent::ProfileSessionFinished { id, .. } => {
            state.status_line = format!("Session finished for '{}'", id.as_str());
        }
        CoreEvent::VaultAttached(id) => {
            state.status_line = format!("Vault '{}' attached", id);
        }
        CoreEvent::VaultDetached(id) => {
            state.status_line = format!("Vault '{}' detached", id);
        }
        CoreEvent::VaultRefreshed(id) => {
            state.status_line = format!("Vault '{}' refreshed", id);
        }
        CoreEvent::VaultInitialized(name) => {
            state.status_line = format!("Vault '{}' initialized", name);
        }
        CoreEvent::ProviderActivated(id) => {
            state.status_line = format!("Provider '{}' activated", id);
        }
        CoreEvent::ProviderDeactivated(id) => {
            state.status_line = format!("Provider '{}' deactivated", id);
        }
        CoreEvent::McpRegistered(name) => {
            state.status_line = format!("MCP '{}' registered", name);
            state.mcp_state.refresh();
        }
        CoreEvent::McpEnabled { name, provider_id } => {
            state.status_line = format!("MCP '{}' enabled for {}", name, provider_id);
            state.mcp_state.refresh();
        }
        CoreEvent::McpDisabled { name, provider_id } => {
            state.status_line = format!("MCP '{}' disabled for {}", name, provider_id);
            state.mcp_state.refresh();
        }
        CoreEvent::McpListed(servers) => {
            state.mcp_state.registry.servers = servers
                .iter()
                .map(|s| (s.name.clone(), s.clone()))
                .collect();
            state.status_line = format!("{} MCP servers listed", servers.len());
        }
        CoreEvent::McpTested {
            name,
            healthy,
            message,
        } => {
            state.status_line = format!(
                "MCP '{}' test: {} ({})",
                name,
                message,
                if *healthy { "healthy" } else { "unhealthy" }
            );
        }
        CoreEvent::AssetInstalled { identity, .. } => {
            state.active_tasks.remove(&0);
            // Keep the spinner in installing_names until ReloadComplete
            // so the UI never shows [ ] between install success and config refresh.
            state.status_line = format!("Asset '{}' installed", identity);
        }
        CoreEvent::AssetRemoved { identity } => {
            state.active_tasks.remove(&0);
            state.status_line = format!("Asset '{}' removed", identity);
        }
        CoreEvent::AssetUpdated { identity } => {
            state.active_tasks.remove(&0);
            state.status_line = format!("Asset '{}' updated", identity);
        }
        CoreEvent::SyncComplete {
            updated,
            skipped,
            errors,
        } => {
            state.active_tasks.remove(&0);
            state.status_line = format!(
                "Sync: {} updated, {} skipped, {} errors",
                updated.len(),
                skipped.len(),
                errors.len()
            );
        }
        CoreEvent::RemoteVaultSearchResults { vault_id, packages } => {
            // Ignore stale results that arrive after the user pressed ESC.
            if !state.search_query.is_empty() {
                state.remote_packages = packages.clone();
                state.status_line = format!("Found {} packages in {}", packages.len(), vault_id);
            }
        }
        CoreEvent::ValidationReport { passed, message } => {
            state.status_line = format!(
                "Validation {}: {}",
                if *passed { "passed" } else { "failed" },
                message
            );
        }
        CoreEvent::TelemetryEnabled => {
            state.analytics_config.settings.enabled = true;
            state.status_line = "Telemetry enabled".to_string();
        }
        CoreEvent::TelemetryDisabled => {
            state.analytics_config.settings.enabled = false;
            state.status_line = "Telemetry disabled".to_string();
        }
        CoreEvent::TelemetryStatusReport(status) => {
            state.analytics_config.settings.enabled = status.enabled;
            state.status_line = format!(
                "Telemetry: {} | Skills: {} | Templates: {} | Profiles: {} | Last scan: {}",
                if status.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                status.skills_tracked,
                status.templates_tracked,
                status.profiles_tracked,
                status.last_scan.as_deref().unwrap_or("never")
            );
        }
        CoreEvent::TelemetryExported { output_path, .. } => {
            state.status_line = match output_path {
                Some(p) => format!("Telemetry exported to {}", p),
                None => "Telemetry exported to stdout".to_string(),
            };
        }
        CoreEvent::Info(msg) => {
            state.status_line = msg.clone();
        }
        CoreEvent::Error(msg) => {
            state.status_line = format!("Error: {}", msg);
        }
        CoreEvent::TaskPhaseChanged { id, phase, .. } => {
            if let Some(task) = state.active_tasks.get_mut(id) {
                task.status = match phase.as_str() {
                    "completed" => crate::tui::progress::ProgressStatus::Running(100),
                    _ => crate::tui::progress::ProgressStatus::Running(0),
                };
            }
        }
        CoreEvent::TaskHungWarning {
            name, elapsed_sec, ..
        } => {
            state.status_line = format!("Warning: task '{}' appears hung ({}s)", name, elapsed_sec);
        }
        CoreEvent::WorkspaceLoaded(_) => {
            state.status_line = "Workspace loaded".to_string();
        }
        CoreEvent::ProfileExported {
            profile_name,
            output_path,
            ..
        } => {
            state.status_line = match output_path {
                Some(p) => format!("Profile '{}' exported to {}", profile_name, p),
                None => format!("Profile '{}' exported", profile_name),
            };
        }
        CoreEvent::ProfileImported { profile_name } => {
            state.status_line = format!("Profile '{}' imported", profile_name);
        }
        CoreEvent::ProfileDiffResult { profile_name, diff } => {
            if diff.has_drift() {
                state.status_line =
                    format!("Profile '{}' has drifted from vault source", profile_name);
            } else {
                state.status_line = format!("Profile '{}' matches vault source", profile_name);
            }
        }
        CoreEvent::TeamInitialized(name) => {
            state.status_line = format!("Team '{}' initialized", name);
        }
        CoreEvent::TeamVaultAdded(identity) => {
            state.status_line = format!("Vault '{}' added to team configuration", identity);
        }
        CoreEvent::TeamRequirementAdded(identity) => {
            state.status_line = format!("Requirement '{}' added to team configuration", identity);
        }
        CoreEvent::TeamRequirementRemoved(identity) => {
            state.status_line = format!("Requirement '{}' removed from team configuration", identity);
        }
        CoreEvent::TeamDiffResult { summary } => {
            state.status_line = summary.clone();
        }
        CoreEvent::TeamStatusResult {
            team_name,
            installed,
            required,
            personal,
        } => {
            state.status_line = format!(
                "Team '{}' status: {}/{} requirements installed, {} personal assets",
                team_name, installed, required, personal
            );
        }
        CoreEvent::TeamSyncComplete {
            vaults_attached,
            skills_installed,
            skills_updated,
            skills_removed_from_team,
            errors,
        } => {
            state.status_line = format!(
                "Team sync: {} vaults attached, {} installed, {} updated, {} removed, {} errors",
                vaults_attached.len(),
                skills_installed.len(),
                skills_updated.len(),
                skills_removed_from_team.len(),
                errors.len()
            );
        }
    }
}
