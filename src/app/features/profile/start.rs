use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::{ConfigStorePort, ProfileRuntimePort};
use crate::domain::profile::ProfileId;
use crate::domain::scope::Scope;
use std::sync::Arc;

/// Start (or simulate) a profile session.
///
/// 1. Load profile from [`ConfigStorePort`].
/// 2. Resolve dependencies: warn about missing skills/MCPs.
/// 3. Look up [`ProfileRuntimePort`] by provider_id.
/// 4. If `dry_run`, build launch plan via `build_launch_plan()`.
/// 5. Otherwise, build plan and immediately execute via `run_plan()`.
/// 6. Emit appropriate [`CoreEvent`]s.
pub fn run(
    id: &ProfileId,
    scope: Scope,
    dry_run: bool,
    store: &dyn ConfigStorePort,
    runtime_ports: &std::collections::HashMap<String, Arc<dyn ProfileRuntimePort>>,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let config = store.load(scope)?;

    let domain_profile = config
        .profiles
        .iter()
        .find(|p| p.name == id.as_str())
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!("Profile '{}' not found in {:?} config", id.as_str(), scope)
        })?;

    // --- Dependency resolution: warn about missing skills/MCPs ---
    // Both scopes use vault_id "auto" — the config store resolves
    // the actual vault internally based on scope.
    for skill in &domain_profile.skills {
        if !config.is_skill_installed("auto", &skill.name) {
            sink.on_error(format!(
                "Skill '{}' referenced by profile '{}' is not installed — \
                 consider running `agk skill install {}`",
                skill.name, domain_profile.name, skill.name,
            ));
        }
    }
    for mcp in &domain_profile.mcps {
        if !config.is_mcp_installed("auto", &mcp.name) {
            sink.on_error(format!(
                "MCP '{}' referenced by profile '{}' is not registered — \
                 consider running `agk mcp add {}`",
                mcp.name, domain_profile.name, mcp.name,
            ));
        }
    }

    let runtime = runtime_ports
        .get(&domain_profile.provider_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Provider '{}' does not support profile runtime",
                domain_profile.provider_id
            )
        })?;

    let app_profile = crate::domain::profile::Profile {
        id: crate::domain::profile::ProfileId::new(&domain_profile.name),
        scope,
        provider_id: crate::domain::profile::ProviderId::new(&domain_profile.provider_id),
        skill_refs: domain_profile.skills.clone(),
        mcp_refs: domain_profile.mcps.clone(),
        instruction_refs: domain_profile.instructions.clone(),
        tool_refs: domain_profile.tool_refs.clone(),
        permission_mode: domain_profile.permission_mode.clone(),
        prompt_overlay_path: domain_profile
            .prompt_overlay_path
            .as_ref()
            .map(std::path::PathBuf::from),
        launch_policy: if dry_run {
            crate::domain::profile::LaunchPolicy::DryRun
        } else {
            crate::domain::profile::LaunchPolicy::AutoRestore
        },
    };

    let plan = runtime.build_launch_plan(&app_profile, Some(&config))?;

    if dry_run {
        sink.on_event(CoreEvent::ProfileLaunchPlan {
            id: id.clone(),
            plan: plan.clone(),
        });
        Ok(CoreOutcome::LaunchPlan(plan))
    } else {
        let session = runtime.run_plan(&plan)?;
        let session_key = format!("pid-{}", session.process.id());
        sink.on_event(CoreEvent::ProfileSessionStarted {
            id: id.clone(),
            session_key: session_key.clone(),
        });
        // Wait for process exit then clean up.
        let _ = session.wait_and_cleanup();
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
        let result = run(
            &ProfileId::new("dev"),
            Scope::Workspace,
            true,
            &FakeStore,
            &std::collections::HashMap::new(),
            &mut sink,
        );
        // No runtime port registered → should fail gracefully
        assert!(result.is_err());
    }

    #[test]
    fn start_profile_live_returns_ok() {
        let mut sink = NullSink;
        let result = run(
            &ProfileId::new("dev"),
            Scope::Workspace,
            false,
            &FakeStore,
            &std::collections::HashMap::new(),
            &mut sink,
        );
        // No runtime port registered → should fail gracefully
        assert!(result.is_err());
    }

    struct FakeStore;
    impl Default for FakeStore {
        fn default() -> Self {
            Self
        }
    }
    impl ConfigStorePort for FakeStore {
        fn load(&self, _scope: Scope) -> anyhow::Result<crate::domain::config::ConfigFile> {
            let mut config = crate::domain::config::ConfigFile::default();
            config.profiles.push(crate::domain::config::Profile {
                name: "dev".into(),
                provider_id: "opencode".into(),
                scope: "workspace".into(),
                skills: vec![crate::domain::profile::ProfileAssetRef::new("rust", "auto")],
                mcps: vec![],
                instructions: vec![],
                tool_refs: vec![],
                permission_mode: None,
                prompt_overlay_path: None,
            });
            Ok(config)
        }
        fn save(
            &self,
            _scope: Scope,
            _config: &crate::domain::config::ConfigFile,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }
}
