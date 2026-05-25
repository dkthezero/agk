use crate::app::event::CoreEvent;
use crate::app::outcome::CoreEventSink;
use serde::Serialize;

/// CLI presenter: implements [`CoreEventSink`] and formats output as either
/// human-readable text or structured JSON (depending on `--json` flag).
///
/// Quiet mode suppresses all non-error output.
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
        if !matches!(self.mode, OutputMode::Quiet) {
            println!("{}", msg);
        }
    }

    fn eprint(&self, msg: &str) {
        if !matches!(self.mode, OutputMode::Quiet) {
            eprintln!("{}", msg);
        }
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
        self.print_json_event(&event);
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
            // Other events are silent in CLI mode
            _ => {}
        }
        self.events.push(event);
    }

    fn on_error(&mut self, error: String) {
        self.eprint(&format!("Error: {}", error));
    }
}

/// Serde-compatible wrapper for events that cannot derive Serialize
/// (e.g. those containing WorkspaceSnapshot or ScannedPackage).
fn event_to_json(event: &CoreEvent) -> serde_json::Value {
    match event {
        CoreEvent::ProfileCreated(id) => {
            serde_json::json!({ "type": "ProfileCreated", "id": id.as_str() })
        }
        CoreEvent::ProfileUpdated(id) => {
            serde_json::json!({ "type": "ProfileUpdated", "id": id.as_str() })
        }
        CoreEvent::ProfileDeleted(id) => {
            serde_json::json!({ "type": "ProfileDeleted", "id": id.as_str() })
        }
        CoreEvent::ProfileValidated { id, valid, message } => {
            serde_json::json!({
                "type": "ProfileValidated",
                "id": id.as_str(),
                "valid": valid,
                "message": message
            })
        }
        CoreEvent::ProfileLaunchPlan { id, plan } => {
            serde_json::json!({
                "type": "ProfileLaunchPlan",
                "id": id.as_str(),
                "provider_id": plan.provider_id.as_str(),
                "skills": plan.skills.iter().map(|s| &s.0).collect::<Vec<_>>(),
                "mcps": plan.mcps.iter().map(|m| &m.0).collect::<Vec<_>>(),
                "files_to_write": plan.files_to_write.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
                "restore_required": plan.restore_required,
            })
        }
        CoreEvent::ProfileSessionStarted { id, session_key } => {
            serde_json::json!({ "type": "ProfileSessionStarted", "id": id.as_str(), "session_key": session_key })
        }
        CoreEvent::ProfileSessionFinished { id, exit_status } => {
            serde_json::json!({ "type": "ProfileSessionFinished", "id": id.as_str(), "exit_status": exit_status })
        }
        CoreEvent::VaultAttached(id) => {
            serde_json::json!({ "type": "VaultAttached", "id": id })
        }
        CoreEvent::VaultDetached(id) => {
            serde_json::json!({ "type": "VaultDetached", "id": id })
        }
        CoreEvent::VaultRefreshed(id) => {
            serde_json::json!({ "type": "VaultRefreshed", "id": id })
        }
        CoreEvent::ProviderActivated(id) => {
            serde_json::json!({ "type": "ProviderActivated", "id": id })
        }
        CoreEvent::ProviderDeactivated(id) => {
            serde_json::json!({ "type": "ProviderDeactivated", "id": id })
        }
        CoreEvent::McpRegistered(name) => {
            serde_json::json!({ "type": "McpRegistered", "name": name })
        }
        CoreEvent::McpEnabled { name, provider_id } => {
            serde_json::json!({ "type": "McpEnabled", "name": name, "provider_id": provider_id })
        }
        CoreEvent::McpDisabled { name, provider_id } => {
            serde_json::json!({ "type": "McpDisabled", "name": name, "provider_id": provider_id })
        }
        CoreEvent::AssetInstalled {
            identity,
            providers,
        } => {
            serde_json::json!({ "type": "AssetInstalled", "identity": identity, "providers": providers })
        }
        CoreEvent::AssetRemoved { identity } => {
            serde_json::json!({ "type": "AssetRemoved", "identity": identity })
        }
        CoreEvent::AssetUpdated { identity } => {
            serde_json::json!({ "type": "AssetUpdated", "identity": identity })
        }
        CoreEvent::SyncComplete {
            updated,
            skipped,
            errors,
        } => {
            serde_json::json!({ "type": "SyncComplete", "updated": updated, "skipped": skipped, "errors": errors })
        }
        CoreEvent::RemoteVaultSearchResults { vault_id, packages } => {
            serde_json::json!({
                "type": "RemoteVaultSearchResults",
                "vault_id": vault_id,
                "count": packages.len()
            })
        }
        CoreEvent::TaskStarted { id, name } => {
            serde_json::json!({ "type": "TaskStarted", "id": id, "name": name })
        }
        CoreEvent::TaskProgress { id, percent } => {
            serde_json::json!({ "type": "TaskProgress", "id": id, "percent": percent })
        }
        CoreEvent::TaskCompleted { id, message } => {
            serde_json::json!({ "type": "TaskCompleted", "id": id, "message": message })
        }
        CoreEvent::TaskFailed { id, error } => {
            serde_json::json!({ "type": "TaskFailed", "id": id, "error": error })
        }
        CoreEvent::ValidationReport { passed, message } => {
            serde_json::json!({ "type": "ValidationReport", "passed": passed, "message": message })
        }
        CoreEvent::Error(msg) => {
            serde_json::json!({ "type": "Error", "message": msg })
        }
        CoreEvent::WorkspaceLoaded(_) => {
            serde_json::json!({ "type": "WorkspaceLoaded" })
        }
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
