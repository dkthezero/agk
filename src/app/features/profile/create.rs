use crate::app::event::{CoreEvent, WorkspaceSnapshot};
use crate::app::features::profile::command::CreateProfileInput;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::{ConfigStorePort, ProcessRunnerPort};
use crate::app::registry::Registry;
use crate::domain::config::Profile;
use crate::domain::profile::{validate_profile_id, validate_profile_refs};

/// Create a new profile in the given scope.
///
/// 1. Validates domain rules (id format, reference validity).
/// 2. Validates provider is active and supports profiles.
/// 3. Loads existing config via [`ConfigStorePort`] and checks uniqueness.
/// 4. Appends the new [`Profile`] to config and saves.
/// 5. For opencode provider, runs `opencode agent create` and copies the generated markdown.
/// 6. Emits [`CoreEvent::ProfileCreated`] + [`CoreEvent::WorkspaceLoaded`].
pub fn run(
    input: &CreateProfileInput,
    store: &dyn ConfigStorePort,
    process_runner: &dyn ProcessRunnerPort,
    registry: &Registry,
    workspace: &std::path::Path,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    // 1. Validate domain rules
    validate_profile_id(&input.id)?;
    validate_profile_refs(&to_domain_profile(input))?;

    // 2. Validate provider
    let provider_id = input.provider_id.as_str();
    let _provider = registry
        .providers
        .iter()
        .find(|p| p.id() == provider_id)
        .and_then(|p| {
            if p.supports_profiles() {
                Some(p.as_ref())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Provider '{}' not active or does not support profiles",
                provider_id
            )
        })?;

    // 3. Load config and check uniqueness
    let mut config = store.load(input.scope)?;
    let id_str = input.id.as_str();
    if config.profiles.iter().any(|p| p.name == id_str) {
        return Err(anyhow::anyhow!(
            "Profile '{}' already exists in {:?} scope",
            id_str,
            input.scope
        ));
    }

    // 4. Build and save (initial placeholder; provider-specific branches
    //    below may rewrite `prompt_overlay_path` after rendering agent
    //    markdown for the `claude-code` provider).
    let profile = Profile {
        name: id_str.to_string(),
        provider_id: provider_id.to_string(),
        scope: input.scope.to_string().to_lowercase(),
        skills: input.skill_refs.clone(),
        mcps: input.mcp_refs.clone(),
        instructions: input.instruction_refs.clone(),
        tool_refs: input.tool_refs.clone(),
        permission_mode: input.permission_mode.clone(),
        prompt_overlay_path: None,
    };
    config.profiles.push(profile);

    // 5. Provider-specific setup.
    #[cfg(feature = "profile-create")]
    let prompt_overlay_path: Option<std::path::PathBuf> = if provider_id == "claude-code" {
        // Build the agent frontmatter from the wizard answer.
        let model = input.model.clone().unwrap_or_else(|| "sonnet".to_string());
        let fm = crate::domain::agent_markdown::AgentFrontmatter {
            name: id_str.to_string(),
            description: input.description.clone(),
            tools: input.tool_refs.clone(),
            disallowed_tools: vec![],
            model: model.clone(),
            permission_mode: input.permission_mode.clone(),
            max_turns: None,
            skills: input.skill_refs.iter().map(|r| r.name.clone()).collect(),
            mcp_servers: input.mcp_refs.iter().map(|r| r.name.clone()).collect(),
            hooks: vec![],
            memory: None,
            background: false,
            effort: None,
            isolation: None,
            color: None,
        };
        // Placeholder: a richer prompt body composition comes in v0.5.
        let prompt_body = input.description.clone();
        let plan = crate::domain::launch_plan::LaunchPlan {
            profile_id: id_str.to_string(),
            provider_id: provider_id.to_string(),
            frontmatter: fm,
            prompt_body,
            resolved_mcp_servers: input.agent_mcp_servers.clone(),
            llm_provider_id: input.llm_provider_id.clone(),
        };
        let md = crate::infra::provider::claude_code::agent_markdown::render_agent_markdown(&plan);
        let agents_dir = workspace.join(".claude").join("agents");
        std::fs::create_dir_all(&agents_dir)?;
        let path = agents_dir.join(format!("{}.md", id_str));
        std::fs::write(&path, md)?;
        sink.on_event(CoreEvent::ProfileCreated(input.id.clone()));
        sink.on_event(CoreEvent::Info(format!(
            "Profile '{}' created. Agent markdown saved to {}",
            id_str,
            path.display()
        )));
        Some(path)
    } else if provider_id == "opencode" {
        run_opencode_branch(input, id_str, workspace, sink, process_runner)?;
        None
    } else {
        anyhow::bail!("unsupported profile provider: {}", provider_id);
    };
    #[cfg(not(feature = "profile-create"))]
    let prompt_overlay_path: Option<std::path::PathBuf> = {
        if provider_id == "claude-code" {
            anyhow::bail!(
                "Provider 'claude-code' requires the 'profile-create' feature \
                 (not enabled in this build)"
            );
        } else if provider_id == "opencode" {
            run_opencode_branch(input, id_str, workspace, sink, process_runner)?;
            None
        } else {
            anyhow::bail!("unsupported profile provider: {}", provider_id);
        }
    };

    // 6. Persist the (possibly rewritten) `prompt_overlay_path` and save.
    config.profiles.last_mut().unwrap().prompt_overlay_path = prompt_overlay_path
        .as_ref()
        .map(|p| p.display().to_string());
    store.save(input.scope, &config)?;

    // 6. Emit workspace snapshot
    let snapshot = WorkspaceSnapshot {
        scope: input.scope,
        profiles: vec![crate::app::snapshot::ProfileEntry {
            name: input.id.as_str().to_string(),
            provider_id: input.provider_id.as_str().to_string(),
            skills: input.skill_refs.clone(),
            mcps: input.mcp_refs.clone(),
            has_drift: false,
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
        tool_refs: input.tool_refs.clone(),
        permission_mode: input.permission_mode.clone(),
        prompt_overlay_path: None,
        launch_policy: crate::domain::profile::LaunchPolicy::default(),
        model: input.model.clone(),
        llm_provider_id: input.llm_provider_id.clone(),
        agent_mcp_servers: input.agent_mcp_servers.clone(),
    }
}

/// Run the opencode branch of profile creation: spawn `opencode agent create`,
/// locate the generated agent markdown, copy it into the profile dir, and emit
/// the standard profile-created events.
fn run_opencode_branch(
    input: &CreateProfileInput,
    id_str: &str,
    workspace: &std::path::Path,
    sink: &mut dyn CoreEventSink,
    process_runner: &dyn ProcessRunnerPort,
) -> CoreResult {
    let profile_dir = workspace.join(".agk").join("profiles").join(id_str);
    std::fs::create_dir_all(&profile_dir)?;

    let profile_dir_str = profile_dir.display().to_string();
    let mut args = vec![
        "agent",
        "create",
        "--path",
        &profile_dir_str,
        "--mode",
        "primary",
        "--name",
        id_str,
    ];
    let desc = input.description.trim();
    let desc_arg;
    if !desc.is_empty() {
        desc_arg = desc.to_string();
        args.push("--description");
        args.push(&desc_arg);
    }

    let _output = process_runner.run("opencode", &args, Some(workspace), None)?;

    // OpenCode `agent create --path <profile_dir>` writes the agent
    // markdown into <profile_dir>/agents/<agent_name>.md.
    let agents_dir = profile_dir.join("agents");
    let source = std::fs::read_dir(&agents_dir).ok().and_then(|entries| {
        entries
            .flatten()
            .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
            .map(|e| e.path())
    });

    let target = profile_dir.join("agent.md");
    if let Some(ref src) = source {
        std::fs::copy(src, &target)?;
        sink.on_event(CoreEvent::ProfileCreated(input.id.clone()));
        sink.on_event(CoreEvent::Info(format!(
            "Profile '{}' created. Agent markdown saved to {}",
            id_str,
            target.display()
        )));
    } else {
        sink.on_event(CoreEvent::ProfileCreated(input.id.clone()));
        sink.on_event(CoreEvent::Info(format!(
            "Profile '{}' created. Agent markdown not found in {}. \
                 You may need to run `opencode agent create` manually.",
            id_str,
            agents_dir.display()
        )));
    }
    Ok(CoreOutcome::Ok)
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

    struct FakeProcessRunner;
    impl ProcessRunnerPort for FakeProcessRunner {
        fn run(
            &self,
            _command: &str,
            _args: &[&str],
            _cwd: Option<&std::path::Path>,
            _env: Option<&[(String, String)]>,
        ) -> anyhow::Result<String> {
            Ok(String::new())
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

    struct FakeProvider;
    impl crate::app::ports::ProviderPort for FakeProvider {
        fn id(&self) -> &str {
            "opencode"
        }
        fn name(&self) -> &str {
            "OpenCode"
        }
        fn install(
            &self,
            _: &crate::domain::asset::ScannedPackage,
            _: crate::domain::scope::Scope,
            _: Option<&crate::domain::config::ConfigFile>,
            _: bool,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _: &crate::domain::identity::AssetIdentity,
            _: &crate::domain::asset::AssetKind,
            _: crate::domain::scope::Scope,
            _: Option<&crate::domain::config::ConfigFile>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn supports_profiles(&self) -> bool {
            true
        }
    }

    fn test_registry() -> Registry {
        let mut r = Registry::new();
        r.register_provider(Box::new(FakeProvider));
        r
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
        let registry = test_registry();
        let result = run(
            &input,
            &store,
            &FakeProcessRunner,
            &registry,
            std::path::Path::new("."),
            &mut sink,
        );
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
        run(
            &input,
            &store,
            &FakeProcessRunner,
            &test_registry(),
            std::path::Path::new("."),
            &mut sink1,
        )
        .unwrap();

        let mut sink2 = NullSink;
        let result = run(
            &input,
            &store,
            &FakeProcessRunner,
            &test_registry(),
            std::path::Path::new("."),
            &mut sink2,
        );
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
        let result = run(
            &input,
            &store,
            &FakeProcessRunner,
            &test_registry(),
            std::path::Path::new("."),
            &mut sink,
        );
        assert!(result.is_err());
    }
}
