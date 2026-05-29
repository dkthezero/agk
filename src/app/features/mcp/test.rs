use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::McpRegistryPort;

/// Test connectivity to a named MCP server.
pub fn run(
    name: &str,
    registry: &dyn McpRegistryPort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
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
            sink.on_event(CoreEvent::McpTested {
                name: name.to_string(),
                healthy: false,
                message: format!("Test failed: {}", e),
            });
            Ok(CoreOutcome::Ok)
        }
    }
}
