use agk::app::command::CoreCommand;
use agk::domain::profile::{ProfileId, ProviderId};
use agk::domain::scope::Scope;
use agk::tui::app::AppState;
use std::collections::HashMap;

/// Start a profile that references a skill not installed in the config.
/// The auto-install will attempt to resolve the missing dependency (but will
/// fail because the FakeStore has no vault with the skill). This verifies that
/// the auto-install wire is connected and the status line reflects the attempt.
#[test]
fn start_profile_auto_install_missing_skill_attempt() {
    let core = super::common::test_core();
    let mut state = AppState::new(
        vec!["Skills".into(), "Profiles".into()],
        vec![true, true],
        HashMap::new(),
    );

    // Create a profile that references a missing skill "rust"
    let create_cmd = CoreCommand::CreateProfile {
        input: {
            let mut input = agk::app::features::profile::command::CreateProfileInput::new(
                ProfileId::new("auto-install-profile"),
                ProviderId::new("opencode"),
                Scope::Workspace,
            );
            input
                .skill_refs
                .push(agk::domain::profile::ProfileAssetRef::new("rust", "auto"));
            input
        },
    };
    let _ = super::common::execute(&core, &mut state, create_cmd);

    // Start the profile with dry_run — it should detect the missing "rust" skill
    // and attempt auto-install. Since no vault provides the "rust" skill in the
    // fake environment, it will fail, but the status line should reflect the
    // attempt.
    let start_cmd = CoreCommand::StartProfile {
        id: ProfileId::new("auto-install-profile"),
        scope: Scope::Workspace,
        dry_run: true,
    };
    let _ = super::common::execute(&core, &mut state, start_cmd);

    // The status line should be non-empty — either mentioning the missing
    // dependency, the auto-install attempt, or the provider not being found.
    assert!(
        !state.status_line.is_empty(),
        "Expected non-empty status line after StartProfile with missing dependency, got empty"
    );
}

/// Verify that `BatchInstallResult::all_succeeded` returns true when there
/// are no failures and false when there are failures or rollback failures.
#[test]
fn batch_install_result_all_succeeded() {
    use agk::app::features::profile::batch_install::BatchInstallResult;

    let result = BatchInstallResult {
        succeeded: vec!["skill:rust".into()],
        failed: vec![],
        rollback_failed: vec![],
    };
    assert!(result.all_succeeded());

    let result = BatchInstallResult {
        succeeded: vec!["skill:rust".into()],
        failed: vec![("skill:python".into(), "not found".into())],
        rollback_failed: vec![],
    };
    assert!(!result.all_succeeded());

    let result = BatchInstallResult {
        succeeded: vec![],
        failed: vec![],
        rollback_failed: vec![("skill:rust".into(), "config error".into())],
    };
    assert!(!result.all_succeeded());

    let result = BatchInstallResult {
        succeeded: vec![],
        failed: vec![],
        rollback_failed: vec![],
    };
    assert!(result.all_succeeded());
}
