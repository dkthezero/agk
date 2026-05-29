use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::McpRegistryPort;

/// List all registered MCP servers.
pub fn run(registry: &dyn McpRegistryPort, sink: &mut dyn CoreEventSink) -> CoreResult {
    let servers = registry.list()?;
    sink.on_event(CoreEvent::McpListed(servers));
    Ok(CoreOutcome::Ok)
}
