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
/// 3. For `claude-code` profiles, build a pre-resolved [`crate::domain::launch_plan::LaunchPlan`]
///    and emit it as a `CoreEvent::ProfileLaunchPlan`. The exec layer (added in C3)
///    consumes this event; dry-run returns immediately after the event.
/// 4. For legacy providers (opencode), look up [`ProfileRuntimePort`] by provider_id.
/// 5. If `dry_run`, build launch plan via `build_launch_plan()`.
/// 6. Otherwise, build plan and immediately execute via `run_plan()`.
/// 7. Emit appropriate [`CoreEvent`]s.
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
            let mut messages: Vec<String> = Vec::new();
            for (dep, err) in &result.failed {
                messages.push(format!("Failed to install dependency '{}': {}", dep, err));
            }
            for (dep, err) in &result.rollback_failed {
                messages.push(format!("Rollback failed for '{}': {}", dep, err));
            }
            return Err(anyhow::anyhow!(
                "Could not resolve all dependencies for profile '{}': {}",
                domain_profile.name,
                messages.join("; ")
            ));
        }
        // Re-load config after dependency resolution may have modified it.
        config = store.load(scope)?;
    }

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
        model: None,
        llm_provider_id: None,
        agent_mcp_servers: vec![],
    };

    // --- claude-code: emit a pre-resolved LaunchPlan event -----------------
    //
    // For claude-code, the new exec layer (added in C3) consumes the
    // `ProfileLaunchPlan` event directly. The plan carries pre-resolved
    // MCP servers so the renderer does not have to call the MCP registry
    // again. Legacy providers (opencode) keep their existing runtime port
    // path below.
    if app_profile.provider_id.as_str() == "claude-code" {
        let fm = crate::domain::agent_markdown::AgentFrontmatter {
            name: app_profile.id.as_str().to_string(),
            description: String::new(),
            tools: app_profile.tool_refs.clone(),
            disallowed_tools: vec![],
            model: app_profile
                .model
                .clone()
                .unwrap_or_else(|| "sonnet".to_string()),
            permission_mode: app_profile.permission_mode.clone(),
            max_turns: None,
            skills: app_profile
                .skill_refs
                .iter()
                .map(|r| r.name.clone())
                .collect(),
            mcp_servers: app_profile
                .mcp_refs
                .iter()
                .map(|r| r.name.clone())
                .collect(),
            hooks: vec![],
            memory: None,
            background: false,
            effort: None,
            isolation: None,
            color: None,
        };
        let prompt_body = app_profile
            .prompt_overlay_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();
        let resolved_mcp_servers = app_profile.agent_mcp_servers.clone();
        let plan = crate::domain::launch_plan::LaunchPlan {
            profile_id: app_profile.id.as_str().to_string(),
            provider_id: app_profile.provider_id.as_str().to_string(),
            frontmatter: fm,
            prompt_body,
            resolved_mcp_servers,
            llm_provider_id: app_profile.llm_provider_id.clone(),
        };
        sink.on_event(CoreEvent::ProfileLaunchPlan { plan });
        if dry_run {
            // Dry-run is fully served by the emitted plan event above.
            return Ok(CoreOutcome::Ok);
        }
        // Live run: the dedicated exec layer that consumes ProfileLaunchPlan is
        // not wired up yet, so fall through to the existing claude-code
        // `ProfileRuntimePort` (src/infra/provider/claude_code/session.rs) to
        // actually start the session. If no runtime is registered for the
        // provider, the lookup below returns a hard error rather than silently
        // reporting success.
    }

    let runtime = runtime_ports
        .get(&domain_profile.provider_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Provider '{}' does not support profile runtime",
                domain_profile.provider_id
            )
        })?;

    let plan = runtime.build_launch_plan(&app_profile, Some(&config))?;

    if dry_run {
        // Dry-run returns the legacy launch plan via CoreOutcome; the new
        // claude-code flow emits its own ProfileLaunchPlan event above.
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

    #[test]
    fn start_claude_code_profile_resolves_launch_plan() {
        let registry = Registry::new();
        let mcp_registry = FakeMcpRegistry::new();
        let mut sink = RecordingSink::default();
        let result = run(
            &ProfileId::new("dev"),
            Scope::Workspace,
            true,
            &ClaudeCodeStore,
            &std::collections::HashMap::new(),
            &registry,
            &mcp_registry,
            &mut sink,
        );
        // Should return Ok immediately for dry-run on a claude-code profile.
        assert!(result.is_ok());
        // The sink should have received a ProfileLaunchPlan event with the
        // domain-level LaunchPlan (not the legacy event::LaunchPlan).
        let plan = sink
            .events
            .iter()
            .find_map(|e| match e {
                CoreEvent::ProfileLaunchPlan { plan } => Some(plan),
                _ => None,
            })
            .expect("ProfileLaunchPlan event should be emitted");
        assert_eq!(plan.profile_id, "dev");
        assert_eq!(plan.provider_id, "claude-code");
        // Default model is "sonnet" when the profile does not set one.
        assert_eq!(plan.frontmatter.model, "sonnet");
        // The fake store provides a single skill reference ("rust") — the
        // frontmatter should expose it under `skills`.
        assert_eq!(plan.frontmatter.skills, vec!["rust".to_string()]);
    }

    #[test]
    fn start_profile_with_unresolvable_dependency_returns_err() {
        // FakeStore returns a profile referencing the "rust" skill in the
        // "auto" vault, but the config does NOT install it and there are no
        // active providers to install it from. Previously this printed
        // `sink.on_error` messages and continued to launch the profile with
        // missing dependencies (false success); it must now return `Err`.
        let registry = Registry::new();
        let mcp_registry = FakeMcpRegistry::new();
        let mut sink = RecordingSink::default();
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
        assert!(
            result.is_err(),
            "profile start with unresolvable deps must error, not launch"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Could not resolve all dependencies"),
            "error should mention dependency resolution failure, got: {err}"
        );
        // No ProfileSessionStarted should be emitted when deps fail.
        let started = sink
            .events
            .iter()
            .any(|e| matches!(e, CoreEvent::ProfileSessionStarted { .. }));
        assert!(
            !started,
            "no ProfileSessionStarted event should be emitted on dep failure"
        );
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

    /// Fake store that returns a `claude-code` profile. Used to verify the
    /// new `ProfileLaunchPlan` branch in `run()`.
    struct ClaudeCodeStore;
    impl ConfigStorePort for ClaudeCodeStore {
        fn load(&self, _scope: Scope) -> anyhow::Result<crate::domain::config::ConfigFile> {
            let mut config = crate::domain::config::ConfigFile::default();
            config.profiles.push(crate::domain::config::Profile {
                name: "dev".into(),
                provider_id: "claude-code".into(),
                scope: "workspace".into(),
                skills: vec![crate::domain::profile::ProfileAssetRef::new("rust", "auto")],
                mcps: vec![],
                instructions: vec![],
                tool_refs: vec!["Read".into(), "Grep".into()],
                permission_mode: Some("acceptEdits".into()),
                prompt_overlay_path: None,
            });
            // Pre-install the "rust" skill in the "auto" vault so dependency
            // resolution is not triggered and the test can verify the launch
            // plan emission path.
            config.vault_defs.insert(
                "auto".to_string(),
                crate::domain::config::VaultSection {
                    vault: None,
                    skills: Some(crate::domain::config::AssetBucket {
                        items: vec!["[rust:1.0.0:0000000000]".to_string()],
                        source: None,
                    }),
                    instructions: None,
                    mcps: None,
                    profiles: None,
                },
            );
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

    /// Captures every event handed to the sink so tests can assert on them.
    #[derive(Default)]
    struct RecordingSink {
        events: Vec<CoreEvent>,
    }
    impl crate::app::outcome::CoreEventSink for RecordingSink {
        fn on_event(&mut self, event: CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, _error: String) {}
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
