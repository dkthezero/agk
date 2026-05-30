use agk::app::command::CoreCommand;
use agk::domain::scope::Scope;
use agk::tui::app::AppState;
use std::collections::HashMap;

#[test]
fn skill_install_updates_status_line() {
    let core = super::common::test_core();
    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());

    let cmd = CoreCommand::InstallAsset {
        identity: "my-skill".into(),
        scope: Scope::Workspace,
        provider_filter: None,
        include_evals: false,
        dry_run: false,
    };

    // InstallAsset may fail because the asset isn't in a vault scan; we only
    // care that the core routes the command and the status line reflects the
    // outcome (success or error) rather than a missing wire.
    let _ = super::common::execute(&core, &mut state, cmd);

    let buf = super::common::render_buffer(&state, 80, 24);
    // The status line should contain *something* related to the install attempt.
    assert!(
        !state.status_line.is_empty(),
        "Expected non-empty status line after InstallAsset"
    );
    super::common::assert_buffer_contains(&buf, &state.status_line);
}
