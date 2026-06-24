//! CLI presenter CoreEventSink implementation.
//!
//! Extracted from `presenter.rs` to keep that file under the 300-LOC
//! ADR-001 §6.4 limit.

use crate::app::event::CoreEvent;
use crate::app::outcome::CoreEventSink;
use crate::cli::presenter::{CliPresenter, OutputMode};

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
                    // The event is accumulated into the JSON batch by
                    // `finalize()`; avoid a duplicate inline print.
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
                } else if matches!(self.mode, OutputMode::Json) {
                    // In JSON mode the event is emitted via the batch in
                    // `finalize()`; avoid a duplicate human-readable line.
                } else {
                    self.eprint(message);
                }
            }
            CoreEvent::ProfileLaunchPlan { plan } => {
                if matches!(self.mode, OutputMode::Json) {
                    // The event is accumulated into the JSON batch by
                    // `finalize()`; avoid a duplicate inline print.
                } else {
                    self.print(&format!("Launch plan for '{}':", plan.profile_id));
                    self.print(&format!("  Provider: {}", plan.provider_id));
                    self.print(&format!("  Model: {}", plan.frontmatter.model));
                    self.print(&format!("  Skills: {:?}", plan.frontmatter.skills));
                    self.print(&format!(
                        "  MCP servers: {:?}",
                        plan.frontmatter.mcp_servers
                    ));
                    self.print(&format!(
                        "  Resolved MCP servers: {}",
                        plan.resolved_mcp_servers.len()
                    ));
                }
            }
            CoreEvent::ValidationReport { passed, message } => {
                self.render_validation_report(*passed, message);
            }
            CoreEvent::TelemetryEnabled => {
                self.print("Telemetry enabled. Background scanner started.");
            }
            CoreEvent::TelemetryDisabled => {
                self.print("Telemetry disabled.");
            }
            CoreEvent::TelemetryStatusReport(status) => {
                if matches!(self.mode, OutputMode::Json) {
                    // The event is accumulated into the JSON batch by
                    // `finalize()`; avoid a duplicate inline print.
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
                } else if matches!(self.mode, OutputMode::Json) {
                    // The event (carrying `content`) is in the JSON batch;
                    // avoid a duplicate inline print of the raw payload.
                } else {
                    self.print(content);
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
                } else if matches!(self.mode, OutputMode::Json) {
                    // The event (carrying `content`) is in the JSON batch;
                    // avoid a duplicate inline print of the raw payload.
                } else {
                    self.print(content);
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
            CoreEvent::LlmProviderListed(cfg) => {
                self.print(&format!(
                    "{} {} -> {}",
                    cfg.id,
                    cfg.kind.as_str(),
                    cfg.endpoint
                ));
            }
            CoreEvent::LlmProviderUpserted(cfg) => {
                self.print(&format!("saved provider '{}'", cfg.id));
            }
            CoreEvent::LlmProviderRemoved(id) => {
                self.print(&format!("removed provider '{}'", id));
            }
            CoreEvent::LlmProviderHealth { id, status } => {
                self.render_llm_health(id, status);
            }
            CoreEvent::Error(msg) => {
                self.render_error_event(msg);
            }
            // Other events are silent in CLI mode
            _ => {}
        }
        self.events.push(event);
    }

    fn on_error(&mut self, error: String) {
        self.render_on_error(error);
    }
}
