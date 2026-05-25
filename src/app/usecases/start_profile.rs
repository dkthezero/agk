use crate::app::event::{CoreEvent, LaunchPlan};
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::domain::profile::ProfileId;
use crate::domain::scope::Scope;

/// Start (or simulate) a profile session.
///
/// Phase 1 stub: returns a [`LaunchPlan`] when `dry_run` is true, otherwise
/// emits a placeholder started event.  In Phase 3 this will resolve the profile
/// from [`ConfigStorePort`], negotiate with [`ProfileRuntimePort`] to build
/// the real plan, and optionally spawn the session.
pub fn run(
    id: &ProfileId,
    _scope: Scope,
    dry_run: bool,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let plan = LaunchPlan {
        profile_id: id.clone(),
        provider_id: crate::domain::profile::ProviderId::new("opencode"),
        ..LaunchPlan::default()
    };

    if dry_run {
        sink.on_event(CoreEvent::ProfileLaunchPlan {
            id: id.clone(),
            plan: plan.clone(),
        });
        Ok(CoreOutcome::LaunchPlan(plan))
    } else {
        sink.on_event(CoreEvent::ProfileSessionStarted {
            id: id.clone(),
            session_key: "stub".into(),
        });
        Ok(CoreOutcome::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::outcome::NullSink;
    use crate::domain::profile::ProfileId;
    use crate::domain::scope::Scope;

    #[test]
    fn start_profile_dry_run_returns_plan() {
        let mut sink = NullSink;
        let result = run(&ProfileId::new("dev"), Scope::Workspace, true, &mut sink);
        assert!(matches!(result, Ok(CoreOutcome::LaunchPlan(_))));
    }

    #[test]
    fn start_profile_live_returns_ok() {
        let mut sink = NullSink;
        let result = run(&ProfileId::new("dev"), Scope::Workspace, false, &mut sink);
        assert!(result.is_ok());
    }
}
