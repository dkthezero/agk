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

    // Encode the transport (and SSE URL, if any) into a single string for
    // the `McpRegistryPort::register` signature, which flattens the typed
    // `McpTransport` to `&str`. The SSE URL is carried as `sse:<url>` so it
    // survives the port boundary; both the infra adapter and the
    // `FakeMcpRegistry::parse_transport` helper decode this form. Without
    // this encoding the SSE URL is silently dropped and the infra adapter
    // falls back to deriving the URL from `args[0]` (a leaky abstraction
    // that pollutes `server.args` with the URL).
    let transport_str = match &input.transport {
        crate::domain::mcp::McpTransport::Stdio => "stdio".to_string(),
        crate::domain::mcp::McpTransport::Sse { url } => format!("sse:{}", url),
    };
    let transport_str = transport_str.as_str();

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
                sink.on_event(CoreEvent::Info(format!(
                    "MCP server '{}' tested successfully",
                    server.name
                )));
            }
            Err(e) => {
                sink.on_event(CoreEvent::Info(format!(
                    "MCP server '{}' registered, but post-registration test failed: {}",
                    server.name, e
                )));
            }
        }
    }
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::event::CoreEvent;
    use crate::app::features::mcp::command::RegisterMcpInput;
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
        fn list(&self) -> anyhow::Result<Vec<crate::domain::mcp::McpServer>> {
            Ok(vec![])
        }
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
                security_flags: vec![],
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

        fn unregister(&self, _name: &str) -> anyhow::Result<()> {
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
    fn register_mcp_with_test_pass_emits_info() {
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
            .any(|e| matches!(e, CoreEvent::Info(ref msg) if msg.contains("tested successfully"))));
    }

    #[test]
    fn register_mcp_with_test_fail_emits_info_warning() {
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
            .any(|e| matches!(e, CoreEvent::Info(ref msg) if msg.contains("registered, but post-registration test failed"))));
    }

    /// Regression: the SSE URL carried on `McpTransport::Sse { url }` must
    /// survive the `McpRegistryPort::register` boundary (which flattens the
    /// typed transport to `&str`). The use case encodes it as `sse:<url>` so
    /// the infra adapter / `FakeMcpRegistry::parse_transport` can decode it;
    /// without this encoding the URL was silently dropped and the infra
    /// adapter derived a bogus URL from `args[0]`.
    #[test]
    fn register_mcp_encodes_sse_url_into_transport_string() {
        use std::sync::{Arc, Mutex};
        struct CapturingRegistry {
            seen_transport: Arc<Mutex<Option<String>>>,
        }
        impl McpRegistryPort for CapturingRegistry {
            fn register(
                &self,
                _name: &str,
                _command: &str,
                _args: Option<&str>,
                _env: Option<&str>,
                transport: &str,
                _description: Option<&str>,
            ) -> anyhow::Result<crate::domain::mcp::McpServer> {
                *self.seen_transport.lock().unwrap() = Some(transport.to_string());
                Ok(crate::domain::mcp::McpServer {
                    name: "captured".to_string(),
                    command: "cmd".to_string(),
                    args: vec![],
                    env: std::collections::HashMap::new(),
                    transport: McpTransport::Stdio,
                    description: None,
                    tested: false,
                    tested_at: None,
                    activation: std::collections::HashMap::new(),
                    security_flags: vec![],
                })
            }
            fn list(&self) -> anyhow::Result<Vec<crate::domain::mcp::McpServer>> {
                Ok(vec![])
            }
            fn test_server(&self, _name: &str) -> anyhow::Result<()> {
                Ok(())
            }
            fn build_providers(
                &self,
                _workspace_root: &std::path::Path,
            ) -> Vec<Box<dyn crate::app::ports::McpProvider>> {
                vec![]
            }
            fn enable(&self, _: &str, _: &str, _: Scope) -> anyhow::Result<()> {
                Ok(())
            }
            fn disable(&self, _: &str, _: &str, _: Scope) -> anyhow::Result<()> {
                Ok(())
            }
            fn unregister(&self, _: &str) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let seen = Arc::new(Mutex::new(None));
        let registry = CapturingRegistry {
            seen_transport: seen.clone(),
        };
        let mut sink = CollectingSink { events: vec![] };
        let input = RegisterMcpInput {
            name: "remote-sse".into(),
            command: "npx".into(),
            args: vec![],
            env: vec![],
            transport: McpTransport::Sse {
                url: "https://mcp.example.com/sse".to_string(),
            },
            description: None,
            test_after: false,
        };
        let result = run(&input, &registry, &mut sink);
        assert!(result.is_ok());
        let captured = seen.lock().unwrap().clone().expect("register was called");
        assert_eq!(
            captured, "sse:https://mcp.example.com/sse",
            "SSE URL must be encoded into the transport string so it survives the port boundary"
        );
    }
}
