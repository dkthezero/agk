//! Integration test for the provider-config-roots feature (technical_design
//! line 105): "enable OpenCode in TUI, select `.agents`, verify skill installs
//! to `.agents/skills/`."
//!
//! This exercises the *real* install path end-to-end through `AgkCore` (the
//! same entry point the TUI runtime loop uses): a real `OpenCodeProvider`
//! writing to a tempdir workspace, with a config that has `provider_roots
//! ["opencode"] = ".agents"` — the exact state the `SelectProviderRoot` modal
//! persists when the user picks the `.agents` option — and a `FakeVault`
//! seeded with an on-disk skill. We then issue `InstallAsset` and assert the
//! skill lands under `.agents/skills/` (not the default `.opencode/skills/`),
//! proving the config-driven root override flows through the install use case
//! into the provider.

use agk::app::command::CoreCommand;
use agk::app::core::AgkCore;
use agk::app::ports::VaultSearchPort;
use agk::app::registry::Registry;
use agk::app::test_support::{
    FakeContextStore, FakeMcpRegistry, FakeProcessRunner, FakeStore, FakeVault,
};
use agk::domain::asset::{AssetKind, ScannedPackage};
use agk::domain::config::ConfigFile;
use agk::domain::identity::AssetIdentity;
use agk::domain::scope::Scope;
use agk::infra::feature::skill::SkillFeatureSet;
use agk::infra::provider::opencode::OpenCodeProvider;
use agk::infra::task_tracker::InMemoryTaskTracker;
use agk::tui::app::AppState;
use agk::tui::core_event_reducer::apply_core_event;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Minimal `VaultSearchPort` fake (only `vault_id` + `search` are needed; the
/// install path never calls `search`).
struct StubVaultSearch;
#[async_trait::async_trait]
impl VaultSearchPort for StubVaultSearch {
    fn vault_id(&self) -> &str {
        "stub"
    }
    async fn search(&self, _query: &str) -> Result<Vec<ScannedPackage>> {
        Ok(vec![])
    }
}

/// Sink that applies CoreEvents directly into AppState (mirrors the
/// `full_flow_tui::common::StateSink` pattern but is local to avoid depending
/// on private helpers).
struct StateSink<'a> {
    state: &'a mut AppState,
}

impl<'a> agk::app::outcome::CoreEventSink for StateSink<'a> {
    fn on_event(&mut self, event: agk::app::event::CoreEvent) {
        apply_core_event(self.state, &event);
    }
    fn on_error(&mut self, error: String) {
        self.state.status_line = format!("Error: {}", error);
    }
}

/// Build a core wired with a real `OpenCodeProvider` rooted at `workspace`,
/// a seeded `FakeVault`, and a pre-seeded `FakeStore` holding the given config.
fn core_with_opencode(
    workspace: PathBuf,
    workspace_config: ConfigFile,
    skill_pkg: ScannedPackage,
) -> AgkCore {
    let store = Arc::new(FakeStore::new());
    store.seed(Scope::Workspace, workspace_config);

    let mut registry = Registry::new();
    registry.register_feature_set(Box::new(SkillFeatureSet));
    let vault = FakeVault::new("workspace");
    vault.seed(skill_pkg);
    registry.register_vault(Box::new(vault));
    registry.register_provider(Box::new(OpenCodeProvider::new(workspace.clone())));

    AgkCore::new(
        store,
        Arc::new(FakeContextStore::new()),
        Arc::new(FakeMcpRegistry::new()),
        Arc::new(StubVaultSearch),
        Arc::new(registry),
        HashMap::new(),
        Arc::new(FakeProcessRunner::new()),
        Arc::new(InMemoryTaskTracker::new()),
        workspace,
        Arc::new(agk::app::test_support::FakeClawHub::new()),
        Arc::new(agk::app::test_support::FakeTeamConfigStore::new()),
    )
}

/// Enable OpenCode with `provider_roots["opencode"] = ".agents"` (the state
/// the `SelectProviderRoot` Enter-handler persists), then install a skill via
/// `InstallAsset` and verify it lands in `.agents/skills/`, not the default
/// `.opencode/skills/`.
#[test]
fn opencode_install_uses_agents_root_when_configured() {
    let workspace = tempfile::tempdir().unwrap();

    // Build a real on-disk skill source the provider can copy from.
    let skill_name = "demo-skill";
    let src = workspace.path().join("source").join(skill_name);
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("SKILL.md"), "# demo\n").unwrap();

    let pkg = ScannedPackage {
        identity: AssetIdentity::new(skill_name, None, "deadbeef00"),
        path: src.clone(),
        vault_id: "workspace".to_string(),
        kind: AssetKind::Skill,
        is_remote: false,
        remote_meta: None,
        requires: vec![],
        requires_optional: vec![],
        author: None,
        description: None,
        include_evals: false,
    };

    let mut config = ConfigFile::default();
    // "Enable OpenCode in TUI": the provider is active for Workspace scope.
    config.providers.push("opencode".to_string());
    // "select `.agents`": the persisted root choice from the modal.
    config
        .provider_roots
        .insert("opencode".to_string(), ".agents".to_string());

    let core = core_with_opencode(workspace.path().to_path_buf(), config, pkg);

    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());

    let cmd = CoreCommand::InstallAsset {
        identity: skill_name.into(),
        scope: Scope::Workspace,
        provider_filter: None,
        include_evals: false,
        dry_run: false,
    };

    let result = {
        let mut sink = StateSink { state: &mut state };
        core.execute(cmd, &mut sink)
    };

    // The install must succeed.
    assert!(
        result.is_ok(),
        "InstallAsset through AgkCore failed: {:?}",
        result.err()
    );

    // The skill must be installed under .agents/skills/ (the configured root),
    // NOT under the default .opencode/skills/.
    let installed = workspace
        .path()
        .join(".agents")
        .join("skills")
        .join(skill_name)
        .join("SKILL.md");
    assert!(
        installed.exists(),
        "Expected skill at {}, but it was not written",
        installed.display()
    );
    let default_root = workspace
        .path()
        .join(".opencode")
        .join("skills")
        .join(skill_name)
        .join("SKILL.md");
    assert!(
        !default_root.exists(),
        "Skill must NOT be installed at the default .opencode root when .agents is configured, but {} exists",
        default_root.display()
    );

    // The TUI reducer must reflect the install in the status line.
    assert!(
        state.status_line.contains("installed"),
        "Expected status line to mention 'installed', got: {}",
        state.status_line
    );
}
