use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::domain::profile::ProfileId;
use crate::domain::scope::Scope;

/// Remove a profile from the config store.
pub fn run(
    id: &ProfileId,
    scope: Scope,
    store: &dyn ConfigStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let mut config = store.load(scope)?;
    let removed = config.remove_profile(id.as_str());

    if removed {
        store.save(scope, &config)?;
        sink.on_event(CoreEvent::ProfileDeleted(id.clone()));
    } else {
        sink.on_error(format!(
            "Profile '{}' not found in {:?}",
            id.as_str(),
            scope
        ));
    }

    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::event::CoreEvent;
    use crate::app::outcome::CoreEventSink;
    use crate::domain::config::ConfigFile;
    use crate::domain::config::Profile;
    use crate::domain::profile::ProfileId;
    use crate::domain::scope::Scope;
    use std::sync::Mutex;

    struct CollectingSink {
        events: Vec<CoreEvent>,
    }

    impl CoreEventSink for CollectingSink {
        fn on_event(&mut self, event: CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, _error: String) {}
    }

    struct FakeStore {
        data: Mutex<ConfigFile>,
    }

    impl ConfigStorePort for FakeStore {
        fn load(&self, _scope: Scope) -> anyhow::Result<ConfigFile> {
            Ok(self.data.lock().unwrap().clone())
        }
        fn save(&self, _scope: Scope, config: &ConfigFile) -> anyhow::Result<()> {
            *self.data.lock().unwrap() = config.clone();
            Ok(())
        }
    }

    #[test]
    fn delete_existing_profile() {
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "test".to_string(),
            provider_id: "opencode".to_string(),
            scope: "workspace".to_string(),
            skills: vec![],
            mcps: vec![],
            instructions: vec![],
            tool_refs: vec![],
            permission_mode: None,
            prompt_overlay_path: None,
        });
        let store = FakeStore {
            data: Mutex::new(config),
        };
        let mut sink = CollectingSink { events: vec![] };
        let result = run(&ProfileId::new("test"), Scope::Workspace, &store, &mut sink);
        assert!(result.is_ok());
        assert!(sink.events.iter().any(|e| matches!(e,
            CoreEvent::ProfileDeleted(ref pid) if pid.as_str() == "test"
        )));
    }

    #[test]
    fn delete_missing_profile_emits_error() {
        let store = FakeStore {
            data: Mutex::new(ConfigFile::default()),
        };
        let mut sink = CollectingSink { events: vec![] };
        let result = run(
            &ProfileId::new("missing"),
            Scope::Workspace,
            &store,
            &mut sink,
        );
        assert!(result.is_ok());
        assert!(!sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::ProfileDeleted(..))));
    }
}
