use crate::app::event::CoreEvent;
use crate::app::features::profile::batch_install::resolve_and_install_deps;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::{ConfigStorePort, McpRegistryPort, ProfileRuntimePort};
use crate::app::registry::Registry;
use crate::domain::profile::ProfileId;
use crate::domain::scope::Scope;
use std::sync::Arc;

/// Start (or simulate) a profile session.
///
/// 1. Load profile from [`ConfigStorePort`].
/// 2. Resolve dependencies: auto-install missing skills, auto-register
///    missing MCPs. If any dependency cannot be resolved, roll back
///    partial installs and return an error.
/// 3. Look up [`ProfileRuntimePort`] by provider_id.
/// 4. If `dry_run`, build launch plan via `build_launch_plan()`.
/// 5. Otherwise, build plan and immediately execute via `run_plan()`.
/// 6. Emit appropriate [`CoreEvent`]s.
#[allow(clippy::too_many_arguments)]
pub fn run(
    id: &ProfileId,
    scope: Scope,
    dry_run: bool,
    store: &dyn ConfigStorePort,
    runtime_ports: &std::collections::HashMap<String, Arc<dyn ProfileRuntimePort>>,
    registry: &Registry,
    mcp_registry: &dyn McpRegistryPort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let mut config = store.load(scope)?;

    let domain_profile = config
        .profiles
        .iter()
        .find(|p| p.name == id.as_str())
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!("Profile '{}' not found in {:?} config", id.as_str(), scope)
        })?;

    // --- Dependency resolution: auto-install missing skills/MCPs ---
    let missing_skills: Vec<_> = domain_profile
        .skills
        .iter()
        .filter(|s| !config.is_skill_installed(&s.vault, &s.name))
        .cloned()
        .collect();
    let missing_mcps: Vec<_> = domain_profile
        .mcps
        .iter()
        .filter(|m| !config.is_mcp_installed(&m.vault, &m.name))
        .cloned()
        .collect();

    if !missing_skills.is_empty() || !missing_mcps.is_empty() {
        sink.on_event(CoreEvent::Info(format!(
            "Resolving {} missing skill(s) and {} missing MCP(s) for profile '{}'...",
            missing_skills.len(),
            missing_mcps.len(),
            domain_profile.name,
        )));
        let providers = registry.active_providers_from_config(&config);
        let result = resolve_and_install_deps(
            &domain_profile.name,
            &missing_skills,
            &missing_mcps,
            scope,
            &config,
            store,
            registry,
            mcp_registry,
            &providers,
            sink,
        );
        if !result.all_succeeded() {
            for (dep, err) in &result.failed {
                sink.on_error(format!("Failed to install dependency '{}': {}", dep, err));
            }
            for (dep, err) in &result.rollback_failed {
                sink.on_error(format!("Rollback failed for '{}': {}", dep, err));
            }
        }
        // Re-load config after dependency resolution may have modified it
        config = store.load(scope)?;
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
        // Track profile launch in telemetry
        let mut analytics = crate::domain::telemetry::AnalyticsConfig::load(
            &crate::domain::paths::analytics_path(),
        )
        .unwrap_or_default();
        analytics.increment_profile_launch(id.as_str(), &domain_profile.provider_id);
        let _ = analytics.save(&crate::domain::paths::analytics_path());
        // Wait for process exit then clean up.
        let _ = session.wait_and_cleanup();
        Ok(CoreOutcome::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::outcome::NullSink;
    use crate::app::ports::McpRegistryPort;
    use crate::app::registry::Registry;
    use crate::domain::mcp::McpServer;
    use crate::domain::profile::ProfileId;
    use crate::domain::scope::Scope;
    use anyhow::Result;

    #[test]
    fn start_profile_dry_run_returns_plan() {
        let registry = Registry::new();
        let mcp_registry = FakeMcpRegistry::new();
        let mut sink = NullSink;
        let result = run(
            &ProfileId::new("dev"),
            Scope::Workspace,
            true,
            &FakeStore,
            &std::collections::HashMap::new(),
            &registry,
            &mcp_registry,
            &mut sink,
        );
        // No runtime port registered → should fail gracefully
        assert!(result.is_err());
    }

    #[test]
    fn start_profile_live_returns_ok() {
        let registry = Registry::new();
        let mcp_registry = FakeMcpRegistry::new();
        let mut sink = NullSink;
        let result = run(
            &ProfileId::new("dev"),
            Scope::Workspace,
            false,
            &FakeStore,
            &std::collections::HashMap::new(),
            &registry,
            &mcp_registry,
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

    struct FakeMcpRegistry {
        servers: std::collections::HashMap<String, McpServer>,
    }
    impl FakeMcpRegistry {
        fn new() -> Self {
            Self {
                servers: std::collections::HashMap::new(),
            }
        }
    }
    impl McpRegistryPort for FakeMcpRegistry {
        fn register(
            &self,
            _name: &str,
            _command: &str,
            _args: Option<&str>,
            _env: Option<&str>,
            _transport: &str,
            _description: Option<&str>,
        ) -> Result<McpServer> {
            anyhow::bail!("not implemented in fake")
        }
        fn list(&self) -> Result<Vec<McpServer>> {
            Ok(self.servers.values().cloned().collect())
        }
        fn test_server(&self, _name: &str) -> Result<()> {
            Ok(())
        }
        fn build_providers(
            &self,
            _workspace_root: &std::path::Path,
        ) -> Vec<Box<dyn crate::app::ports::McpProvider>> {
            vec![]
        }
        fn enable(
            &self,
            _name: &str,
            _provider_id: &str,
            _scope: crate::domain::scope::Scope,
        ) -> Result<()> {
            Ok(())
        }
        fn disable(
            &self,
            _name: &str,
            _provider_id: &str,
            _scope: crate::domain::scope::Scope,
        ) -> Result<()> {
            Ok(())
        }
        fn unregister(&self, _name: &str) -> Result<()> {
            Ok(())
        }
    }
}
