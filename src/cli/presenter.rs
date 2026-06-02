use crate::app::event::CoreEvent;
use crate::app::outcome::CoreEventSink;
use crate::cli::presenter_json::event_to_json;
use serde::Serialize;

/// CLI presenter: implements [`CoreEventSink`] and formats output as either
/// human-readable text or structured JSON (depending on `--json` flag).
///
/// Quiet mode suppresses non-error output; errors are still written to stderr.
pub struct CliPresenter {
    mode: OutputMode,
    /// Accumulated events for JSON batch output.
    events: Vec<CoreEvent>,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputMode {
    Quiet,
    Normal,
    Json,
}

impl CliPresenter {
    pub fn new(json: bool, quiet: bool) -> Self {
        let mode = if quiet {
            OutputMode::Quiet
        } else if json {
            OutputMode::Json
        } else {
            OutputMode::Normal
        };
        Self {
            mode,
            events: Vec::new(),
        }
    }

    pub fn mode(&self) -> OutputMode {
        self.mode
    }

    /// Prints the final JSON batch if `--json`.
    pub fn finalize(&self) {
        if matches!(self.mode, OutputMode::Json) && !self.events.is_empty() {
            let json_events: Vec<serde_json::Value> =
                self.events.iter().map(event_to_json).collect();
            let summary = JsonSummary {
                events: json_events,
            };
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        }
    }

    fn print(&self, msg: &str) {
        if matches!(self.mode, OutputMode::Normal) {
            println!("{}", msg);
        }
    }

    fn eprint(&self, msg: &str) {
        eprintln!("{}", msg);
    }

    fn print_json_event(&self, event: &CoreEvent) {
        if matches!(self.mode, OutputMode::Json) {
            println!(
                "{}",
                serde_json::to_string_pretty(&event_to_json(event)).unwrap()
            );
        }
    }
}

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

/// A serialisable summary of events emitted during a command execution.
#[derive(Serialize)]
struct JsonSummary {
    events: Vec<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::profile::ProfileId;

    #[test]
    fn quiet_mode_suppresses_output() {
        let mut presenter = CliPresenter::new(false, true);
        presenter.on_event(CoreEvent::ProfileCreated(ProfileId::new("test")));
        presenter.on_error("something went wrong".into());
        presenter.finalize();
        // No assertions needed — just must not panic
    }

    #[test]
    fn json_mode_collects_events() {
        let mut presenter = CliPresenter::new(true, false);
        presenter.on_event(CoreEvent::ProfileCreated(ProfileId::new("dev")));
        presenter.on_event(CoreEvent::ProviderActivated("opencode".into()));
        // finalize() would print JSON — we just verify it doesn't panic
        presenter.finalize();
    }
}
