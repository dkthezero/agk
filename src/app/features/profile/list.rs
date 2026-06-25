use crate::app::bootstrap::build_profile_entries;
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::domain::scope::Scope;

/// List all configured profiles for the given scope.
///
/// Emits a single [`CoreEvent::ProfileListed`] carrying the display view
/// models built by [`build_profile_entries`]. A missing config file is
/// treated as an empty profile set (the store returns `Ok(default)` in
/// that case), while a malformed config surfaces as an error — per the
/// AGENTS.md "Malformed Config: Surface Errors, Don't Default" rule.
pub fn run(scope: Scope, store: &dyn ConfigStorePort, sink: &mut dyn CoreEventSink) -> CoreResult {
    let config = store.load(scope)?;
    let entries = build_profile_entries(&config);
    sink.on_event(CoreEvent::ProfileListed(entries));
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::ConfigFile;
    use std::sync::Mutex;

    struct FakeStore {
        data: Mutex<ConfigFile>,
    }

    impl FakeStore {
        fn with(file: ConfigFile) -> Self {
            Self {
                data: Mutex::new(file),
            }
        }
    }

    impl ConfigStorePort for FakeStore {
        fn load(&self, _scope: Scope) -> anyhow::Result<ConfigFile> {
            Ok(self.data.lock().unwrap().clone())
        }
        fn save(&self, _scope: Scope, _config: &ConfigFile) -> anyhow::Result<()> {
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
    fn list_profiles_empty_config_emits_empty_list_event() {
        let store = FakeStore::with(ConfigFile::default());
        let mut sink = CollectingSink { events: vec![] };
        let result = run(Scope::Workspace, &store, &mut sink);
        assert!(result.is_ok());
        assert_eq!(sink.events.len(), 1);
        match &sink.events[0] {
            CoreEvent::ProfileListed(entries) => assert!(entries.is_empty()),
            other => panic!("expected ProfileListed, got {:?}", other),
        }
    }

    #[test]
    fn list_profiles_emits_one_entry_per_configured_profile() {
        let mut file = ConfigFile::default();
        file.profiles.push(crate::domain::config::Profile {
            name: "dev".to_string(),
            provider_id: "opencode".to_string(),
            ..Default::default()
        });
        file.profiles.push(crate::domain::config::Profile {
            name: "backend".to_string(),
            provider_id: "claude".to_string(),
            skills: vec![crate::domain::profile::ProfileAssetRef::new("rust", "auto")],
            ..Default::default()
        });
        let store = FakeStore::with(file);
        let mut sink = CollectingSink { events: vec![] };
        run(Scope::Workspace, &store, &mut sink).unwrap();

        let entries = match &sink.events[0] {
            CoreEvent::ProfileListed(e) => e,
            other => panic!("expected ProfileListed, got {:?}", other),
        };
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "dev");
        assert_eq!(entries[0].provider_id, "opencode");
        assert_eq!(entries[1].name, "backend");
        assert_eq!(entries[1].provider_id, "claude");
        assert_eq!(entries[1].skills.len(), 1);
    }

    #[test]
    fn list_profiles_surfaces_malformed_config_error() {
        struct FailingStore;
        impl ConfigStorePort for FailingStore {
            fn load(&self, _scope: Scope) -> anyhow::Result<ConfigFile> {
                Err(anyhow::anyhow!("malformed config"))
            }
            fn save(&self, _scope: Scope, _config: &ConfigFile) -> anyhow::Result<()> {
                Ok(())
            }
        }
        let store = FailingStore;
        let mut sink = CollectingSink { events: vec![] };
        let result = run(Scope::Workspace, &store, &mut sink);
        assert!(result.is_err());
        assert!(sink.events.is_empty());
    }
}
