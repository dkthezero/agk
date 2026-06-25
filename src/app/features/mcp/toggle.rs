use crate::app::core::AgkCore;
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::domain::scope::Scope;

/// Toggle an MCP server across all MCP-capable active providers.
///
/// For each active provider that supports MCP, enables the server if it is
/// currently disabled for the given scope, or disables it if enabled.
/// Emits [`CoreEvent::McpEnabled`] / [`CoreEvent::McpDisabled`] per provider.
pub fn run(name: &str, scope: Scope, core: &AgkCore, sink: &mut dyn CoreEventSink) -> CoreResult {
    let config = core.store.load(scope)?;
    let providers = core.mcp_registry.build_providers(&core.workspace_root);
    let supported_ids: std::collections::HashSet<&str> =
        providers.iter().map(|p| p.provider_id()).collect();

    let target_providers: Vec<String> = config
        .providers
        .iter()
        .filter(|pid| supported_ids.contains(pid.as_str()))
        .cloned()
        .collect();

    if target_providers.is_empty() {
        sink.on_event(CoreEvent::Info(
            "No MCP-capable providers active. Activate Claude Code or OpenCode in Providers tab."
                .to_string(),
        ));
        return Ok(CoreOutcome::Ok);
    }

    // Load current MCP registry to determine per-provider enablement
    let servers = core.mcp_registry.list()?;
    let server = servers.iter().find(|s| s.name == name);

    let mut failures: Vec<String> = Vec::new();
    for pid in &target_providers {
        let is_enabled = server
            .and_then(|s| s.activation.get(pid))
            .map(|a| match scope {
                Scope::Global => a.global,
                Scope::Workspace => a.workspace,
            })
            .unwrap_or(false);

        let result = if is_enabled {
            core.mcp_registry.disable(name, pid, scope)
        } else {
            core.mcp_registry.enable(name, pid, scope)
        };

        match result {
            Ok(_) => {
                if is_enabled {
                    sink.on_event(CoreEvent::McpDisabled {
                        name: name.to_string(),
                        provider_id: pid.clone(),
                    });
                } else {
                    sink.on_event(CoreEvent::McpEnabled {
                        name: name.to_string(),
                        provider_id: pid.clone(),
                    });
                }
            }
            Err(e) => {
                let msg = format!("Failed to toggle MCP '{}' for {}: {}", name, pid, e);
                sink.on_event(CoreEvent::Error(msg.clone()));
                failures.push(msg);
            }
        }
    }

    if !failures.is_empty() {
        return Err(anyhow::anyhow!(
            "Failed to toggle MCP '{}' for {} provider(s): {}",
            name,
            failures.len(),
            failures.join("; ")
        ));
    }

    Ok(CoreOutcome::Ok)
}
