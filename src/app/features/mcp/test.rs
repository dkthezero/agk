use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::McpRegistryPort;

/// Test connectivity to a named MCP server.
///
/// Emits a `McpTested` event (carrying `healthy` + a human-readable
/// `message`) for renderers, and returns `Err` when the test failed so the
/// CLI dispatcher maps the failure to a non-zero exit code.  Returning
/// `Ok(CoreOutcome::Ok)` on failure would make `agk mcp test <name>` exit 0
/// despite reporting "Test failed: ..." — a false-success the
/// `TaskFailed`-then-`Ok` anti-pattern documented in AGENTS.md.
pub fn run(name: &str, registry: &dyn McpRegistryPort, sink: &mut dyn CoreEventSink) -> CoreResult {
    match registry.test_server(name) {
        Ok(()) => {
            sink.on_event(CoreEvent::McpTested {
                name: name.to_string(),
                healthy: true,
                message: format!("MCP server '{}' is healthy", name),
            });
            Ok(CoreOutcome::Ok)
        }
        Err(e) => {
            let message = format!("Test failed: {}", e);
            sink.on_event(CoreEvent::McpTested {
                name: name.to_string(),
                healthy: false,
                message,
            });
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::outcome::NullSink;
    use crate::app::test_support::collecting_sink::CollectingSink;
    use crate::app::test_support::fake_mcp_registry::FakeMcpRegistry;
    use anyhow::anyhow;

    #[test]
    fn run_healthy_emits_healthy_event_and_returns_ok() {
        let mut sink = CollectingSink::new();
        let reg = FakeMcpRegistry::new();
        let result = run("any", &reg, &mut sink);
        assert!(result.is_ok(), "healthy test must return Ok");
        assert_eq!(sink.events.len(), 1);
        match &sink.events[0] {
            CoreEvent::McpTested { healthy, .. } => assert!(*healthy),
            other => panic!("expected McpTested, got {:?}", other),
        }
    }

    /// Regression: `mcp test` must return `Err` (so the CLI exits non-zero)
    /// when the connectivity probe fails, while still emitting the
    /// `McpTested { healthy: false }` event for renderers.  Previously it
    /// returned `Ok(CoreOutcome::Ok)`, making `agk mcp test <name>` exit 0
    /// despite printing "Test failed: ..." — a false success.
    #[test]
    fn run_unhealthy_emits_event_and_returns_err() {
        let mut sink = CollectingSink::new();
        let reg = FakeMcpRegistry::new();
        *reg.on_test.lock().unwrap() = Box::new(|_| Err(anyhow!("boom")));
        let result = run("dead", &reg, &mut sink);
        assert!(result.is_err(), "unhealthy test must return Err");
        assert_eq!(sink.events.len(), 1, "the failure event must still emit");
        match &sink.events[0] {
            CoreEvent::McpTested {
                healthy, message, ..
            } => {
                assert!(!*healthy);
                assert!(message.contains("Test failed: boom"));
            }
            other => panic!("expected McpTested, got {:?}", other),
        }
    }

    #[test]
    fn run_unhealthy_with_null_sink_still_returns_err() {
        let mut sink = NullSink;
        let reg = FakeMcpRegistry::new();
        *reg.on_test.lock().unwrap() = Box::new(|_| Err(anyhow!("boom")));
        assert!(run("dead", &reg, &mut sink).is_err());
    }
}
