use crate::app::features::profile::command::CreateProfileInput;
use crate::app::event::{CoreEvent, WorkspaceSnapshot};
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::domain::config::Profile;
use crate::domain::profile::{validate_profile_id, validate_profile_refs};

/// Create a new profile in the given scope.
///
/// 1. Validates domain rules (id format, reference validity).
/// 2. Loads existing config via [`ConfigStorePort`].
/// 3. Ensures the profile id is unique in the scope.
/// 4. Appends the new [`Profile`] to config and saves.
/// 5. Emits [`CoreEvent::ProfileCreated`] + [`CoreEvent::WorkspaceLoaded`].
pub fn run(
    input: &CreateProfileInput,
    store: &dyn ConfigStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    // 1. Validate domain rules
    validate_profile_id(&input.id)?;
    validate_profile_refs(&to_domain_profile(input))?;

    // 2. Load config and check uniqueness
    let mut config = store.load(input.scope)?;
    let id_str = input.id.as_str();
    if config.profiles.iter().any(|p| p.name == id_str) {
        return Err(anyhow::anyhow!(
            "Profile '{}' already exists in {:?} scope",
            id_str,
            input.scope
        ));
    }

    // 3. Build and save
    let profile = Profile {
        name: id_str.to_string(),
        provider_id: input.provider_id.as_str().to_string(),
        skills: input.skill_refs.iter().map(|s| s.0.clone()).collect(),
        mcps: input.mcp_refs.iter().map(|m| m.0.clone()).collect(),
    };
    config.profiles.push(profile);
    store.save(input.scope, &config)?;

    // 4. Emit events
    sink.on_event(CoreEvent::ProfileCreated(input.id.clone()));

    let snapshot = WorkspaceSnapshot {
        scope: input.scope,
        profiles: vec![crate::app::snapshot::ProfileEntry {
            name: input.id.as_str().to_string(),
            provider_id: input.provider_id.as_str().to_string(),
            skills: input.skill_refs.iter().map(|s| s.0.clone()).collect(),
            mcps: input.mcp_refs.iter().map(|m| m.0.clone()).collect(),
        }],
        ..WorkspaceSnapshot::default()
    };
    sink.on_event(CoreEvent::WorkspaceLoaded(snapshot));

    Ok(CoreOutcome::Ok)
}

fn to_domain_profile(input: &CreateProfileInput) -> crate::domain::profile::Profile {
    crate::domain::profile::Profile {
        id: input.id.clone(),
        scope: input.scope,
        provider_id: input.provider_id.clone(),
        skill_refs: input.skill_refs.clone(),
        mcp_refs: input.mcp_refs.clone(),
        instruction_refs: input.instruction_refs.clone(),
        prompt_overlay_path: None,
        launch_policy: crate::domain::profile::LaunchPolicy::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::outcome::{CoreEventSink, NullSink};
    use crate::domain::profile::ProfileId;
    use crate::domain::scope::Scope;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeStore {
        data: Mutex<HashMap<String, crate::domain::config::ConfigFile>>,
    }

    impl FakeStore {
        fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    impl ConfigStorePort for FakeStore {
        fn load(&self, scope: Scope) -> anyhow::Result<crate::domain::config::ConfigFile> {
            Ok(self
                .data
                .lock()
                .unwrap()
                .get(&format!("{:?}", scope))
                .cloned()
                .unwrap_or_default())
        }
        fn save(
            &self,
            scope: Scope,
            config: &crate::domain::config::ConfigFile,
        ) -> anyhow::Result<()> {
            self.data
                .lock()
                .unwrap()
                .insert(format!("{:?}", scope), config.clone());
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
    fn create_profile_saves_to_store() {
        let store = FakeStore::new();
        let mut sink = CollectingSink { events: vec![] };
        let input = CreateProfileInput::new(
            ProfileId::new("test-profile"),
            crate::domain::profile::ProviderId::new("opencode"),
            Scope::Workspace,
        );
        let result = run(&input, &store, &mut sink);
        assert!(result.is_ok());

        let config = store.load(Scope::Workspace).unwrap();
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].name, "test-profile");
        assert_eq!(config.profiles[0].provider_id, "opencode");

        assert!(sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::ProfileCreated(id) if id.as_str() == "test-profile")));
        assert!(sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::WorkspaceLoaded(_))));
    }

    #[test]
    fn duplicate_profile_id_fails() {
        let store = FakeStore::new();
        let input = CreateProfileInput::new(
            ProfileId::new("dup"),
            crate::domain::profile::ProviderId::new("opencode"),
            Scope::Workspace,
        );
        let mut sink1 = NullSink;
        run(&input, &store, &mut sink1).unwrap();

        let mut sink2 = NullSink;
        let result = run(&input, &store, &mut sink2);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn invalid_profile_id_fails() {
        let store = FakeStore::new();
        let mut sink = NullSink;
        let input = CreateProfileInput::new(
            ProfileId::new("foo/bar"),
            crate::domain::profile::ProviderId::new("opencode"),
            Scope::Workspace,
        );
        assert!(run(&input, &store, &mut sink).is_err());
    }
}
