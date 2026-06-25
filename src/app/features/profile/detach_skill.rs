use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::domain::profile::{ProfileId, SkillId};
use crate::domain::scope::Scope;

/// Detach a skill reference from a profile.
pub fn run(
    profile_id: &ProfileId,
    skill_id: &SkillId,
    scope: Scope,
    store: &dyn ConfigStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let mut config = store.load(scope)?;
    let profile_name = profile_id.as_str();
    let skill_ref = skill_id.as_str().to_string();

    if let Some(profile) = config.profiles.iter_mut().find(|p| p.name == profile_name) {
        let before = profile.skills.len();
        profile.skills.retain(|s| s.name != skill_ref);
        if profile.skills.len() < before {
            store.save(scope, &config)?;
            sink.on_event(CoreEvent::ProfileUpdated(profile_id.clone()));
            Ok(CoreOutcome::Ok)
        } else {
            Err(anyhow::anyhow!(
                "Skill '{}' not found in profile '{}'",
                skill_id.as_str(),
                profile_name
            ))
        }
    } else {
        Err(anyhow::anyhow!(
            "Profile '{}' not found in {:?}",
            profile_name,
            scope
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::event::CoreEvent;
    use crate::app::outcome::CoreEventSink;
    use crate::domain::config::{ConfigFile, Profile};
    use crate::domain::profile::{ProfileId, SkillId};
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
    fn detach_existing_skill() {
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "test".to_string(),
            provider_id: "opencode".to_string(),
            scope: "workspace".to_string(),
            skills: vec![crate::domain::profile::ProfileAssetRef::new("rust", "auto")],
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
        let result = run(
            &ProfileId::new("test"),
            &SkillId::new("rust"),
            Scope::Workspace,
            &store,
            &mut sink,
        );
        assert!(result.is_ok());
        assert!(sink.events.iter().any(|e| matches!(e,
            CoreEvent::ProfileUpdated(ref pid) if pid.as_str() == "test"
        )));
    }

    #[test]
    fn detach_missing_skill_emits_error() {
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
        let result = run(
            &ProfileId::new("test"),
            &SkillId::new("rust"),
            Scope::Workspace,
            &store,
            &mut sink,
        );
        assert!(result.is_err());
        assert!(!sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::ProfileUpdated(..))));
    }
}
