//! CLI presenter JSON serializer.
//!
//! `event_to_json` was extracted from `presenter.rs` to keep that file
//! under the 300-LOC ADR-001 §6.4 limit. The function is consumed only by
//! `presenter.rs::CliPresenter::finalize`; everything else is internal.

use crate::app::event::CoreEvent;

/// Serde-compatible wrapper for events that cannot derive Serialize
/// (e.g. those containing WorkspaceSnapshot or ScannedPackage).
pub(crate) fn event_to_json(event: &CoreEvent) -> serde_json::Value {
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
        CoreEvent::ProfileExported {
            profile_name,
            content,
            output_path,
        } => {
            serde_json::json!({
                "type": "ProfileExported",
                "profile_name": profile_name,
                "content": content,
                "output_path": output_path,
            })
        }
        CoreEvent::ProfileImported { profile_name } => {
            serde_json::json!({ "type": "ProfileImported", "profile_name": profile_name })
        }
        CoreEvent::ProfileDiffResult { profile_name, diff } => {
            serde_json::json!({
                "type": "ProfileDiffResult",
                "profile_name": profile_name,
                "has_drift": diff.has_drift(),
                "added_skills": diff.added_skills.iter().map(|r| &r.name).collect::<Vec<_>>(),
                "removed_skills": diff.removed_skills.iter().map(|r| &r.name).collect::<Vec<_>>(),
                "added_mcps": diff.added_mcps.iter().map(|r| &r.name).collect::<Vec<_>>(),
                "removed_mcps": diff.removed_mcps.iter().map(|r| &r.name).collect::<Vec<_>>(),
                "added_instructions": diff.added_instructions.iter().map(|r| &r.name).collect::<Vec<_>>(),
                "removed_instructions": diff.removed_instructions.iter().map(|r| &r.name).collect::<Vec<_>>(),
                "added_tools": &diff.added_tools,
                "removed_tools": &diff.removed_tools,
                "permission_mode_differs": diff.permission_mode_differs,
            })
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
        CoreEvent::McpListed(servers) => {
            serde_json::json!({
                "type": "McpListed",
                "servers": servers.iter().map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "command": s.command,
                        "transport": match s.transport {
                            crate::domain::mcp::McpTransport::Stdio => "stdio",
                            crate::domain::mcp::McpTransport::Sse { .. } => "sse",
                        },
                        "tested": s.tested,
                        "tested_at": s.tested_at,
                        "security_flags": s.security_flags.iter().map(|f| {
                            serde_json::json!({
                                "flag": f,
                                "severity": f.severity(),
                                "badge": f.badge(),
                                "description": f.description(),
                            })
                        }).collect::<Vec<_>>(),
                    })
                }).collect::<Vec<_>>()
            })
        }
        CoreEvent::McpTested {
            name,
            healthy,
            message,
        } => {
            serde_json::json!({ "type": "McpTested", "name": name, "healthy": healthy, "message": message })
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
        CoreEvent::TelemetryEnabled => {
            serde_json::json!({ "type": "TelemetryEnabled" })
        }
        CoreEvent::TelemetryDisabled => {
            serde_json::json!({ "type": "TelemetryDisabled" })
        }
        CoreEvent::TelemetryStatusReport(status) => {
            serde_json::json!({
                "type": "TelemetryStatusReport",
                "enabled": status.enabled,
                "skills_tracked": status.skills_tracked,
                "templates_tracked": status.templates_tracked,
                "profiles_tracked": status.profiles_tracked,
                "last_scan": status.last_scan
            })
        }
        CoreEvent::TelemetryExported {
            content,
            output_path,
        } => {
            serde_json::json!({
                "type": "TelemetryExported",
                "content": content,
                "output_path": output_path
            })
        }
        CoreEvent::Info(msg) => {
            serde_json::json!({ "type": "Info", "message": msg })
        }
        CoreEvent::Error(msg) => {
            serde_json::json!({ "type": "Error", "message": msg })
        }
        CoreEvent::TaskPhaseChanged {
            id,
            phase,
            elapsed_ms,
        } => {
            serde_json::json!({ "type": "TaskPhaseChanged", "id": id, "phase": phase, "elapsed_ms": elapsed_ms })
        }
        CoreEvent::TaskHungWarning {
            id,
            name,
            elapsed_sec,
        } => {
            serde_json::json!({ "type": "TaskHungWarning", "id": id, "name": name, "elapsed_sec": elapsed_sec })
        }
        CoreEvent::WorkspaceLoaded(_) => {
            serde_json::json!({ "type": "WorkspaceLoaded" })
        }
    }
}
