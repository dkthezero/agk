use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::McpRegistryPort;
use crate::domain::scope::Scope;

/// Disable an MCP server for a specific provider and scope.
pub fn run(
    name: &str,
    provider_id: &str,
    scope: Scope,
    mcp_registry: &dyn McpRegistryPort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    if let Err(e) = mcp_registry.disable(name, provider_id, scope) {
        sink.on_event(CoreEvent::Error(format!(
            "Failed to disable MCP '{}' for provider '{}': {}",
            name, provider_id, e
        )));
        return Ok(CoreOutcome::Ok);
    }

    sink.on_event(CoreEvent::McpDisabled {
        name: name.to_string(),
        provider_id: provider_id.to_string(),
    });
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::event::CoreEvent;
    use crate::app::outcome::CoreEventSink;
    use crate::domain::scope::Scope;

    struct CollectingSink {
        events: Vec<CoreEvent>,
    }

    impl CoreEventSink for CollectingSink {
        fn on_event(&mut self, event: CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, _error: String) {}
    }

    struct FakeMcpRegistry {
        fail: bool,
    }

    impl McpRegistryPort for FakeMcpRegistry {
        fn register(
            &self,
            _name: &str,
            _command: &str,
            _args: Option<&str>,
            _env: Option<&str>,
            _transport: &str,
            _description: Option<&str>,
        ) -> anyhow::Result<crate::domain::mcp::McpServer> {
            unreachable!()
        }

        fn test_server(&self, _name: &str) -> anyhow::Result<()> {
            unreachable!()
        }

        fn build_providers(
            &self,
            _workspace_root: &std::path::Path,
        ) -> Vec<Box<dyn crate::app::ports::McpProvider>> {
            vec![]
        }

        fn enable(&self, _name: &str, _provider_id: &str, _scope: Scope) -> anyhow::Result<()> {
            Ok(())
        }

        fn disable(&self, _name: &str, _provider_id: &str, _scope: Scope) -> anyhow::Result<()> {
            if self.fail {
                Err(anyhow::anyhow!("disable failed"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn disable_mcp_emits_disabled_event() {
        let mut sink = CollectingSink { events: vec![] };
        let registry = FakeMcpRegistry { fail: false };
        let result = run("github", "opencode", Scope::Workspace, &registry, &mut sink);
        assert!(result.is_ok());
        assert!(sink.events.iter().any(|e| matches!(e,
            CoreEvent::McpDisabled { name, provider_id } if name == "github" && provider_id == "opencode"
        )));
    }

    #[test]
    fn disable_mcp_failure_emits_error() {
        let mut sink = CollectingSink { events: vec![] };
        let registry = FakeMcpRegistry { fail: true };
        let result = run("github", "opencode", Scope::Workspace, &registry, &mut sink);
        assert!(result.is_ok());
        assert!(sink.events.iter().any(|e| matches!(e,
            CoreEvent::Error(msg) if msg.contains("disable failed")
        )));
    }
}
