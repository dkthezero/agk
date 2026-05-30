use agk::app::command::CoreCommand;
use agk::domain::scope::Scope;
use agk::tui::app::AppState;
use std::collections::HashMap;

#[test]
fn provider_activate_and_deactivate_updates_status_line() {
    let core = super::common::test_core();
    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());

    let activate = CoreCommand::ActivateProvider {
        id: "opencode".into(),
        scope: Scope::Workspace,
    };
    super::common::execute(&core, &mut state, activate).unwrap();
    assert!(
        state.status_line.contains("activated"),
        "Expected 'activated' in status line, got: {}",
        state.status_line
    );

    let deactivate = CoreCommand::DeactivateProvider {
        id: "opencode".into(),
        scope: Scope::Workspace,
    };
    super::common::execute(&core, &mut state, deactivate).unwrap();
    assert!(
        state.status_line.contains("deactivated"),
        "Expected 'deactivated' in status line, got: {}",
        state.status_line
    );

    let buf = super::common::render_buffer(&state, 80, 24);
    super::common::assert_buffer_contains(&buf, "deactivated");
}
