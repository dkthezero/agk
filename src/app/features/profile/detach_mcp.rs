use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::domain::profile::{McpServerId, ProfileId};
use crate::domain::scope::Scope;

/// Detach an MCP server reference from a profile.
pub fn run(
    profile_id: &ProfileId,
    mcp_id: &McpServerId,
    scope: Scope,
    store: &dyn ConfigStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let mut config = store.load(scope)?;
    let profile_name = profile_id.as_str();
    let mcp_ref = mcp_id.as_str().to_string();

    if let Some(profile) = config.profiles.iter_mut().find(|p| p.name == profile_name) {
        let before = profile.mcps.len();
        profile.mcps.retain(|m| m != &mcp_ref);
        if profile.mcps.len() < before {
            store.save(scope, &config)?;
            sink.on_event(CoreEvent::ProfileUpdated(profile_id.clone()));
        } else {
            sink.on_error(format!(
                "MCP '{}' not found in profile '{}'",
                mcp_id.as_str(),
                profile_name
            ));
        }
    } else {
        sink.on_error(format!(
            "Profile '{}' not found in {:?}",
            profile_name, scope
        ));
    }

    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::event::CoreEvent;
    use crate::app::outcome::CoreEventSink;
    use crate::domain::config::{ConfigFile, Profile};
    use crate::domain::profile::{McpServerId, ProfileId};
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
    fn detach_existing_mcp() {
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "test".to_string(),
            provider_id: "opencode".to_string(),
            skills: vec![],
            mcps: vec!["github".to_string()],
        });
        let store = FakeStore {
            data: Mutex::new(config),
        };
        let mut sink = CollectingSink { events: vec![] };
        let result = run(
            &ProfileId::new("test"),
            &McpServerId::new("github"),
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
    fn detach_missing_mcp_emits_error() {
        let mut config = ConfigFile::default();
        config.profiles.push(Profile {
            name: "test".to_string(),
            provider_id: "opencode".to_string(),
            skills: vec![],
            mcps: vec![],
        });
        let store = FakeStore {
            data: Mutex::new(config),
        };
        let mut sink = CollectingSink { events: vec![] };
        let result = run(
            &ProfileId::new("test"),
            &McpServerId::new("github"),
            Scope::Workspace,
            &store,
            &mut sink,
        );
        assert!(result.is_ok());
        assert!(!sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::ProfileUpdated(..))));
    }
}
