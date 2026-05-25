use crate::app::command::CreateProfileInput;
use crate::app::event::{CoreEvent, WorkspaceSnapshot};
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::domain::profile::{validate_profile_id, validate_profile_refs};

/// Create a new profile in the given scope.
///
/// In Phase 3 this will:
/// 1. Load config via [`ConfigStorePort`].
/// 2. Validate that the ID is unique.
/// 3. Save the profile.
///
/// For Phase 1 the stub validates domain rules and emits events.
pub fn run(input: &CreateProfileInput, sink: &mut dyn CoreEventSink) -> CoreResult {
    // 1. Validate domain rules
    validate_profile_id(&input.id)?;
    validate_profile_refs(&to_domain_profile(input))?;

    // 2. Emit event (placeholder — in Phase 3 the config store actually saves)
    sink.on_event(CoreEvent::ProfileCreated(input.id.clone()));

    // 3. Return placeholder snapshot (Phase 3 loads from store)
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
    use crate::app::command::CreateProfileInput;
    use crate::app::outcome::{CoreEventSink, NullSink};
    use crate::domain::profile::ProfileId;
    use crate::domain::scope::Scope;

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
    fn create_profile_emits_events() {
        let mut sink = CollectingSink { events: vec![] };
        let input = CreateProfileInput::new(
            ProfileId::new("test-profile"),
            crate::domain::profile::ProviderId::new("opencode"),
            Scope::Workspace,
        );
        let result = run(&input, &mut sink);
        assert!(result.is_ok());

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
    fn invalid_profile_id_fails() {
        let mut sink = NullSink;
        let input = CreateProfileInput::new(
            ProfileId::new("foo/bar"),
            crate::domain::profile::ProviderId::new("opencode"),
            Scope::Workspace,
        );
        assert!(run(&input, &mut sink).is_err());
    }
}
