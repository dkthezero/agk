use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ContextStorePort;
use crate::app::snapshot::ContextEntry;

/// List all available contexts, emitting a single [`CoreEvent::ContextListed`]
/// event carrying one [`ContextEntry`] per context (with the active one flagged).
pub fn run(context_store: &dyn ContextStorePort, sink: &mut dyn CoreEventSink) -> CoreResult {
    let file = context_store.load_contexts()?;
    let entries: Vec<ContextEntry> = file
        .contexts
        .iter()
        .map(|(name, ctx)| ContextEntry {
            name: name.clone(),
            display_name: ctx.display_name.clone(),
            is_active: name == &file.current_context,
            environment: ctx.environment.map(|e| e.as_str().to_string()),
            vaults: ctx.vaults.clone(),
            profiles: ctx.profiles.clone(),
            providers: ctx.providers.clone(),
        })
        .collect();
    sink.on_event(CoreEvent::ContextListed(entries));
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::outcome::CoreEventSink;
    use crate::app::ports::ContextStorePort;
    use crate::domain::context::{ContextConfig, ContextFile};
    use std::sync::Mutex;

    struct FakeCtxStore {
        file: Mutex<ContextFile>,
    }

    impl FakeCtxStore {
        fn with(file: ContextFile) -> Self {
            Self {
                file: Mutex::new(file),
            }
        }
    }

    impl ContextStorePort for FakeCtxStore {
        fn load_contexts(&self) -> anyhow::Result<ContextFile> {
            Ok(self.file.lock().unwrap().clone())
        }
        fn save_contexts(&self, _file: &ContextFile) -> anyhow::Result<()> {
            Ok(())
        }
        fn current_context(&self) -> anyhow::Result<crate::domain::context::ContextId> {
            Ok(crate::domain::context::ContextId::new(
                self.file.lock().unwrap().current_context.clone(),
            ))
        }
        fn switch_context(&self, id: &crate::domain::context::ContextId) -> anyhow::Result<()> {
            self.file.lock().unwrap().current_context = id.as_str().to_string();
            Ok(())
        }
    }

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
    fn list_contexts_emits_single_context_listed_event() {
        let mut file = ContextFile::default();
        file.ensure_default();
        file.contexts.insert(
            "company-x".to_string(),
            ContextConfig {
                display_name: Some("Company X".to_string()),
                vaults: vec!["team".to_string()],
                profiles: vec!["backend".to_string()],
                environment: Some(crate::domain::context::Environment::Prod),
                ..ContextConfig::default()
            },
        );
        let store = FakeCtxStore::with(file);
        let mut sink = CollectingSink { events: vec![] };
        let result = run(&store, &mut sink);
        assert!(result.is_ok());

        // Exactly one ContextListed event carrying all contexts.
        assert_eq!(sink.events.len(), 1);
        let entries = match &sink.events[0] {
            CoreEvent::ContextListed(e) => e.clone(),
            other => panic!("expected ContextListed, got {:?}", other),
        };
        assert_eq!(entries.len(), 2); // default + company-x

        let personal = entries.iter().find(|e| e.name == "default").unwrap();
        assert!(personal.is_active);
        assert_eq!(personal.display_name.as_deref(), Some("Personal"));
        let company = entries.iter().find(|e| e.name == "company-x").unwrap();
        assert!(!company.is_active);
        assert_eq!(company.display_name.as_deref(), Some("Company X"));
        assert_eq!(company.environment.as_deref(), Some("prod"));
        assert_eq!(company.vaults, vec!["team"]);
        assert_eq!(company.profiles, vec!["backend"]);
    }

    #[test]
    fn list_contexts_empty_file_emits_empty_list() {
        let mut file = ContextFile::default();
        file.contexts.clear();
        file.current_context.clear();
        let store = FakeCtxStore::with(file);
        let mut sink = CollectingSink { events: vec![] };
        let result = run(&store, &mut sink);
        assert!(result.is_ok());
        let entries = match &sink.events[0] {
            CoreEvent::ContextListed(e) => e.clone(),
            _ => panic!("expected ContextListed"),
        };
        assert!(entries.is_empty());
    }
}
