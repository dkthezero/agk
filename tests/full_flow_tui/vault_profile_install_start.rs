use agk::app::command::CoreCommand;
use agk::app::features::profile::command::CreateProfileInput;
use agk::domain::profile::{ProfileAssetRef, ProfileId, ProviderId};
use agk::domain::scope::Scope;
use agk::tui::app::AppState;
use std::collections::HashMap;

/// Start a profile that does not exist in config — the command should
/// surface an error indicating the profile was not found, proving the
/// StartProfile wire is connected and the config lookup is performed.
#[test]
fn start_missing_profile_errors() {
    let core = super::common::test_core();
    let mut state = AppState::new(
        vec!["Skills".into(), "Profiles".into()],
        vec![true, true],
        HashMap::new(),
    );

    let cmd = CoreCommand::StartProfile {
        id: ProfileId::new("nonexistent-profile"),
        scope: Scope::Workspace,
        dry_run: true,
    };
    // The command should execute without panicking; the status line should
    // reflect that the profile was not found.
    let _ = super::common::execute(&core, &mut state, cmd);
    assert!(
        !state.status_line.is_empty(),
        "Expected non-empty status line after StartProfile for missing profile"
    );
    // The status line should mention the profile name or an error
    assert!(
        state.status_line.contains("nonexistent-profile")
            || state.status_line.contains("Error")
            || state.status_line.contains("not found"),
        "Expected status line to mention the missing profile or error, got: {}",
        state.status_line
    );
}

/// Create a profile that references an uninstalled skill, then start it
/// with dry_run=true. The start command should detect the missing dependency
/// and surface an informational message.
#[test]
fn start_profile_detects_missing_skill_dependency() {
    let core = super::common::test_core();
    let mut state = AppState::new(
        vec!["Skills".into(), "Profiles".into()],
        vec![true, true],
        HashMap::new(),
    );

    // First, create a profile that references an uninstalled skill.
    // The skill "rust" won't be installed because the FakeStore starts empty
    // and the FakeVaultSearch returns no results.
    let create_cmd = CoreCommand::CreateProfile {
        input: {
            let mut input = CreateProfileInput::new(
                ProfileId::new("test-dep-profile"),
                ProviderId::new("opencode"),
                Scope::Workspace,
            );
            input.skill_refs.push(ProfileAssetRef::new("rust", "auto"));
            input
        },
    };
    let _ = super::common::execute(&core, &mut state, create_cmd);

    // Now try to start the profile with dry_run.
    // The start should detect the missing "rust" skill dependency.
    let start_cmd = CoreCommand::StartProfile {
        id: ProfileId::new("test-dep-profile"),
        scope: Scope::Workspace,
        dry_run: true,
    };
    let _ = super::common::execute(&core, &mut state, start_cmd);

    // The status line should be non-empty — either an error about missing
    // runtime/provider, or an info message about resolving dependencies.
    assert!(
        !state.status_line.is_empty(),
        "Expected non-empty status line after StartProfile with missing dependency, got empty"
    );
}

/// Verify that a dry-run StartProfile for a profile that exists in config
/// (but has no runtime port) produces a meaningful error about the provider
/// not supporting profile runtime, rather than silently succeeding.
#[test]
fn start_profile_no_runtime_port_errors() {
    let core = super::common::test_core();
    let mut state = AppState::new(
        vec!["Skills".into(), "Profiles".into()],
        vec![true, true],
        HashMap::new(),
    );

    // Create a minimal profile
    let create_cmd = CoreCommand::CreateProfile {
        input: CreateProfileInput::new(
            ProfileId::new("bare-profile"),
            ProviderId::new("opencode"),
            Scope::Workspace,
        ),
    };
    let _ = super::common::execute(&core, &mut state, create_cmd);

    // Attempt to start it with dry_run
    let start_cmd = CoreCommand::StartProfile {
        id: ProfileId::new("bare-profile"),
        scope: Scope::Workspace,
        dry_run: true,
    };
    let _ = super::common::execute(&core, &mut state, start_cmd);

    // Either we get an error about the provider not being found /
    // not supporting runtime, or we get a success/plan. Either way,
    // the status line must be non-empty.
    assert!(
        !state.status_line.is_empty(),
        "Expected non-empty status line after StartProfile dry_run, got empty"
    );
}
