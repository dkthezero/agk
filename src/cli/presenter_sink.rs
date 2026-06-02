//! CLI presenter CoreEventSink implementation.
//!
//! Extracted from `presenter.rs` to keep that file under the 300-LOC
//! ADR-001 §6.4 limit.

use crate::app::event::CoreEvent;
use crate::app::outcome::CoreEventSink;
use crate::cli::presenter::{CliPresenter, OutputMode};
use crate::cli::presenter_json::event_to_json;

impl CoreEventSink for CliPresenter {
    fn on_event(&mut self, event: CoreEvent) {
        match &event {
            CoreEvent::TaskStarted { id, name } => {
                self.print(&format!("[{}] Starting: {}", id, name));
            }
            CoreEvent::TaskProgress { id, percent } => {
                self.print(&format!("[{}] Progress: {}%", id, percent));
            }
            CoreEvent::TaskCompleted { id, message } => {
                self.print(&format!("[{}] Completed: {}", id, message));
            }
            CoreEvent::TaskFailed { id, error } => {
                self.eprint(&format!("[{}] Failed: {}", id, error));
            }
            CoreEvent::ProfileCreated(id) => {
                self.print(&format!("Profile '{}' created", id.as_str()));
            }
            CoreEvent::ProfileDeleted(id) => {
                self.print(&format!("Profile '{}' deleted", id.as_str()));
            }
            CoreEvent::ProviderActivated(id) => {
                self.print(&format!("Provider '{}' activated", id));
            }
            CoreEvent::ProviderDeactivated(id) => {
                self.print(&format!("Provider '{}' deactivated", id));
            }
            CoreEvent::VaultAttached(id) => {
                self.print(&format!("Vault '{}' attached", id));
            }
            CoreEvent::VaultDetached(id) => {
                self.print(&format!("Vault '{}' detached", id));
            }
            CoreEvent::VaultInitialized(name) => {
                self.print(&format!("Vault '{}' initialized", name));
            }
            CoreEvent::McpRegistered(name) => {
                self.print(&format!("MCP server '{}' registered", name));
            }
            CoreEvent::McpEnabled { name, provider_id } => {
                self.print(&format!(
                    "MCP server '{}' enabled for {}",
                    name, provider_id
                ));
            }
            CoreEvent::McpDisabled { name, provider_id } => {
                self.print(&format!(
                    "MCP server '{}' disabled for {}",
                    name, provider_id
                ));
            }
            CoreEvent::McpListed(servers) => {
                if matches!(self.mode, OutputMode::Json) {
                    self.print_json_event(&event);
                } else if servers.is_empty() {
                    self.print("No MCP servers registered.");
                } else {
                    for s in servers {
                        let tested = if s.tested { "[✓]" } else { "[ ]" };
                        let transport = match s.transport {
                            crate::domain::mcp::McpTransport::Stdio => "stdio",
                            crate::domain::mcp::McpTransport::Sse { .. } => "sse",
                        };
                        self.print(&format!("{} {} ({})", tested, s.name, transport));
                    }
                }
            }
            CoreEvent::McpTested {
                healthy, message, ..
            } => {
                if *healthy {
                    self.print(message);
                } else {
                    self.eprint(message);
                }
            }
            CoreEvent::ProfileLaunchPlan { id, plan } => {
                if matches!(self.mode, OutputMode::Json) {
                    self.print(
                        &serde_json::to_string_pretty(&event_to_json(
                            &CoreEvent::ProfileLaunchPlan {
                                id: id.clone(),
                                plan: plan.clone(),
                            },
                        ))
                        .unwrap(),
                    );
                } else {
                    self.print(&format!("Launch plan for '{}':", id.as_str()));
                    self.print(&format!("  Provider: {}", plan.provider_id.as_str()));
                    self.print(&format!("  Skills: {:?}", plan.skills));
                    self.print(&format!("  MCPs: {:?}", plan.mcps));
                    self.print(&format!("  Files to write: {:?}", plan.files_to_write));
                }
            }
            CoreEvent::ValidationReport { passed, message } => {
                if *passed {
                    self.print(&format!("Validation passed: {}", message));
                } else {
                    self.eprint(&format!("Validation failed: {}", message));
                }
            }
            CoreEvent::TelemetryEnabled => {
                self.print("Telemetry enabled. Background scanner started.");
            }
            CoreEvent::TelemetryDisabled => {
                self.print("Telemetry disabled.");
            }
            CoreEvent::TelemetryStatusReport(status) => {
                if matches!(self.mode, OutputMode::Json) {
                    self.print_json_event(&CoreEvent::TelemetryStatusReport(status.clone()));
                } else {
                    self.print(&format!(
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
                    ));
                }
            }
            CoreEvent::TelemetryExported {
                content,
                output_path,
            } => {
                if let Some(path) = output_path {
                    if let Err(e) = std::fs::write(path, content) {
                        self.eprint(&format!("Failed to write export file: {}", e));
                    } else {
                        self.print(&format!("Telemetry exported to {}", path));
                    }
                } else {
                    println!("{}", content);
                }
            }
            CoreEvent::Info(msg) => {
                self.print(msg);
            }
            CoreEvent::TeamInitialized(name) => {
                self.print(&format!("Team '{}' initialized", name));
            }
            CoreEvent::TeamVaultAdded(identity) => {
                self.print(&format!("Vault '{}' added to team configuration", identity));
            }
            CoreEvent::TeamRequirementAdded(identity) => {
                self.print(&format!(
                    "Requirement '{}' added to team configuration",
                    identity
                ));
            }
            CoreEvent::TeamRequirementRemoved(identity) => {
                self.print(&format!(
                    "Requirement '{}' removed from team configuration",
                    identity
                ));
            }
            CoreEvent::TeamDiffResult { summary } => {
                self.print(summary);
            }
            CoreEvent::TeamStatusResult {
                team_name,
                installed,
                required,
                personal,
            } => {
                self.print(&format!(
                    "Team '{}' status: {}/{} requirements installed, {} personal assets",
                    team_name, installed, required, personal
                ));
            }
            CoreEvent::TeamSyncComplete {
                vaults_attached,
                skills_installed,
                skills_updated,
                skills_removed_from_team,
                errors,
            } => {
                if !vaults_attached.is_empty() {
                    self.print(&format!(
                        "Team vaults attached: {}",
                        vaults_attached.join(", ")
                    ));
                }
                if !skills_installed.is_empty() {
                    self.print(&format!(
                        "Team skills installed: {}",
                        skills_installed.join(", ")
                    ));
                }
                if !skills_updated.is_empty() {
                    self.print(&format!(
                        "Team skills updated: {}",
                        skills_updated.join(", ")
                    ));
                }
                if !skills_removed_from_team.is_empty() {
                    self.print(&format!(
                        "Team skills removed from requirements: {}",
                        skills_removed_from_team.join(", ")
                    ));
                }
                if !errors.is_empty() {
                    for err in errors {
                        self.eprint(&format!("Team sync error: {}", err));
                    }
                }
            }
            CoreEvent::ProfileExported {
                profile_name,
                content,
                output_path,
            } => {
                if let Some(path) = output_path {
                    if let Err(e) = std::fs::write(path, content) {
                        self.eprint(&format!("Failed to write export file: {}", e));
                    } else {
                        self.print(&format!("Profile '{}' exported to {}", profile_name, path));
                    }
                } else {
                    println!("{}", content);
                    self.print(&format!("Profile '{}' exported", profile_name));
                }
            }
            CoreEvent::ProfileImported { profile_name } => {
                self.print(&format!("Profile '{}' imported", profile_name));
            }
            CoreEvent::ProfileDiffResult { profile_name, diff } => {
                self.print(&format!("Profile: {}", profile_name));
                self.print(&diff.summary());
            }
            CoreEvent::TaskHungWarning {
                id,
                name,
                elapsed_sec,
            } => {
                self.eprint(&format!(
                    "[HUNG] Task {} '{}' has been running for {}s",
                    id, name, elapsed_sec
                ));
            }
            // Other events are silent in CLI mode
            _ => {}
        }
        self.events.push(event);
    }

    fn on_error(&mut self, error: String) {
        self.eprint(&format!("Error: {}", error));
    }
}
