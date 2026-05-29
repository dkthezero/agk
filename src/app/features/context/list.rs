use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ContextStorePort;

/// List all available contexts, emitting one [`CoreEvent::TaskCompleted`]
/// per context, with the active one marked by an asterisk.
pub fn run(context_store: &dyn ContextStorePort, sink: &mut dyn CoreEventSink) -> CoreResult {
    let file = context_store.load_contexts()?;
    for (name, ctx) in &file.contexts {
        let marker = if name == &file.current_context {
            "* "
        } else {
            "  "
        };
        let display = ctx.display_name.as_ref().unwrap_or(name);
        let env = ctx
            .environment
            .map(|e| e.as_str().to_string())
            .unwrap_or_default();
        sink.on_event(CoreEvent::TaskCompleted {
            id: 0,
            message: format!(
                "{}{} [{}] (vaults: {}, profiles: {})",
                marker,
                display,
                env,
                ctx.vaults.len(),
                ctx.profiles.len()
            ),
        });
    }
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
    fn list_contexts_emits_one_completion_per_context() {
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
        assert_eq!(sink.events.len(), 2); // default + company-x

        let messages: Vec<String> = sink
            .events
            .iter()
            .filter_map(|e| {
                if let CoreEvent::TaskCompleted { message, .. } = e {
                    Some(message.clone())
                } else {
                    None
                }
            })
            .collect();

        assert!(messages.iter().any(|m| m.contains("* Personal")));
        assert!(messages.iter().any(|m| m.contains("Company X [prod]")));
    }
}
