use agk::app::command::CoreCommand;
use agk::domain::scope::Scope;
use agk::tui::app::AppState;
use std::collections::HashMap;

#[test]
fn sync_assets_updates_status_line() {
    let core = super::common::test_core();
    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());

    let cmd = CoreCommand::SyncAssets {
        scope: Scope::Workspace,
        dry_run: false,
    };

    // Sync may return an error if no vaults are configured; we still assert
    // that the status line is populated (proving the wire is connected).
    let _ = super::common::execute(&core, &mut state, cmd);

    assert!(
        !state.status_line.is_empty(),
        "Expected non-empty status line after SyncAssets, got empty"
    );

    let buf = super::common::render_buffer(&state, 80, 24);
    super::common::assert_buffer_contains(&buf, &state.status_line);
}
