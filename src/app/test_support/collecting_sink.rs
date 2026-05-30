use crate::app::event::CoreEvent;
use crate::app::outcome::CoreEventSink;

/// A [`CoreEventSink`] that records every event and error into `Vec`s so
/// tests can assert on the exact sequence produced by a use case or adapter.
#[derive(Debug, Default)]
pub struct CollectingSink {
    pub events: Vec<CoreEvent>,
    pub errors: Vec<String>,
}

impl CollectingSink {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CoreEventSink for CollectingSink {
    fn on_event(&mut self, event: CoreEvent) {
        self.events.push(event);
    }
    fn on_error(&mut self, error: String) {
        self.errors.push(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collecting_sink_records_events() {
        let mut sink = CollectingSink::new();
        sink.on_event(CoreEvent::VaultAttached("test".into()));
        assert_eq!(sink.events.len(), 1);
        assert!(matches!(&sink.events[0], CoreEvent::VaultAttached(id) if id == "test"));
    }

    #[test]
    fn collecting_sink_records_errors() {
        let mut sink = CollectingSink::new();
        sink.on_error("something broke".into());
        assert_eq!(sink.errors.len(), 1);
        assert_eq!(sink.errors[0], "something broke");
    }
}
