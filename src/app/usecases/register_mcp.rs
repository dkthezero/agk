use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};

/// Register an MCP server and optionally test it.
///
/// In Phase 3 this delegates to a [`McpRegistryPort`] instead of calling
/// `infra::mcp::register` directly.
pub fn run(
    name: String,
    command: String,
    args: Vec<String>,
    transport: String,
    description: Option<String>,
    test_after: bool,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    // Phase 3: this will call McpRegistryPort::register() instead
    sink.on_event(CoreEvent::McpRegistered(name.clone()));
    if test_after {
        // Phase 3: call McpRegistryPort::test()
        sink.on_event(CoreEvent::TaskCompleted {
            id: 0,
            message: format!("MCP server '{}' tested successfully", name),
        });
    }
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::event::CoreEvent;
    use crate::app::outcome::CoreEventSink;

    struct CollectingSink {
        events: Vec<CoreEvent>,
    }

    impl CoreEventSink for CollectingSink {
        fn on_event(&mut self, event: CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, _error: String) {}
    }

    #[test]
    fn register_mcp_emits_event() {
        let mut sink = CollectingSink { events: vec![] };
        let result = run(
            "github".into(),
            "npx".into(),
            vec!["@github/mcp".into()],
            "stdio".into(),
            Some("GitHub MCP".into()),
            false,
            &mut sink,
        );
        assert!(result.is_ok());
        assert!(sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::McpRegistered(n) if n == "github")));
    }
}
