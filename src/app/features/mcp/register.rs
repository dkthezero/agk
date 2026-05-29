use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::McpRegistryPort;

/// Register an MCP server and optionally test it.
///
/// Delegates to [`McpRegistryPort`] instead of calling infra directly.
pub fn run(
    input: &crate::app::features::mcp::command::RegisterMcpInput,
    mcp_registry: &dyn McpRegistryPort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let args_joined = if input.args.is_empty() {
        None
    } else {
        Some(input.args.join(" "))
    };
    let args_str = args_joined.as_deref();

    let env_joined = if input.env.is_empty() {
        None
    } else {
        Some(
            input
                .env
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(","),
        )
    };
    let env_str = env_joined.as_deref();

    let transport_str = match &input.transport {
        crate::domain::mcp::McpTransport::Stdio => "stdio",
        crate::domain::mcp::McpTransport::Sse { .. } => "sse",
    };

    let server = mcp_registry.register(
        &input.name,
        &input.command,
        args_str,
        env_str,
        transport_str,
        input.description.as_deref(),
    )?;

    sink.on_event(CoreEvent::McpRegistered(server.name.clone()));

    if input.test_after {
        match mcp_registry.test_server(&server.name) {
            Ok(_) => {
                sink.on_event(CoreEvent::TaskCompleted {
                    id: 0,
                    message: format!("MCP server '{}' tested successfully", server.name),
                });
            }
            Err(e) => {
                sink.on_event(CoreEvent::TaskFailed {
                    id: 0,
                    error: format!("MCP server '{}' test failed: {}", server.name, e),
                });
            }
        }
    }
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::mcp::command::RegisterMcpInput;
    use crate::app::event::CoreEvent;
    use crate::app::outcome::CoreEventSink;
    use crate::domain::mcp::McpTransport;
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
        should_pass_test: bool,
    }

    impl McpRegistryPort for FakeMcpRegistry {
        fn register(
            &self,
            name: &str,
            _command: &str,
            _args: Option<&str>,
            _env: Option<&str>,
            _transport: &str,
            description: Option<&str>,
        ) -> anyhow::Result<crate::domain::mcp::McpServer> {
            Ok(crate::domain::mcp::McpServer {
                name: name.to_string(),
                command: "cmd".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                transport: McpTransport::Stdio,
                description: description.map(|d| d.to_string()),
                tested: false,
                tested_at: None,
                activation: std::collections::HashMap::new(),
            })
        }

        fn test_server(&self, _name: &str) -> anyhow::Result<()> {
            if self.should_pass_test {
                Ok(())
            } else {
                Err(anyhow::anyhow!("test fail"))
            }
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
            Ok(())
        }
    }

    #[test]
    fn register_mcp_emits_registered_event() {
        let mut sink = CollectingSink { events: vec![] };
        let registry = FakeMcpRegistry {
            should_pass_test: true,
        };
        let input = RegisterMcpInput {
            name: "github".into(),
            command: "npx".into(),
            args: vec!["@github/mcp".into()],
            env: vec![],
            transport: McpTransport::Stdio,
            description: Some("GitHub MCP".into()),
            test_after: false,
        };
        let result = run(&input, &registry, &mut sink);
        assert!(result.is_ok());
        assert!(sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::McpRegistered(n) if n == "github")));
    }

    #[test]
    fn register_mcp_with_test_pass_emits_completed() {
        let mut sink = CollectingSink { events: vec![] };
        let registry = FakeMcpRegistry {
            should_pass_test: true,
        };
        let input = RegisterMcpInput {
            name: "github".into(),
            command: "npx".into(),
            args: vec!["@github/mcp".into()],
            env: vec![],
            transport: McpTransport::Stdio,
            description: Some("GitHub MCP".into()),
            test_after: true,
        };
        let result = run(&input, &registry, &mut sink);
        assert!(result.is_ok());
        assert!(sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::TaskCompleted { ref message, .. } if message.contains("tested successfully"))));
    }

    #[test]
    fn register_mcp_with_test_fail_emits_failed() {
        let mut sink = CollectingSink { events: vec![] };
        let registry = FakeMcpRegistry {
            should_pass_test: false,
        };
        let input = RegisterMcpInput {
            name: "github".into(),
            command: "npx".into(),
            args: vec!["@github/mcp".into()],
            env: vec![],
            transport: McpTransport::Stdio,
            description: Some("GitHub MCP".into()),
            test_after: true,
        };
        let result = run(&input, &registry, &mut sink);
        assert!(result.is_ok());
        assert!(sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::TaskFailed { ref error, .. } if error.contains("test failed"))));
    }
}
