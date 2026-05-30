use agk::app::command::CoreCommand;
use agk::app::features::vault::command::AttachVaultInput;
use agk::domain::config::{LocalVaultSource, VaultConfig};
use agk::domain::scope::Scope;
use agk::tui::app::AppState;
use std::collections::HashMap;

#[test]
fn vault_attach_updates_status_line() {
    let core = super::common::test_core();
    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());

    let cmd = CoreCommand::AttachVault {
        input: AttachVaultInput {
            vault_id: "test-vault".into(),
            config: VaultConfig::Local(LocalVaultSource {
                path: "/tmp".into(),
            }),
            scope: Scope::Workspace,
        },
    };

    super::common::execute(&core, &mut state, cmd).unwrap();
    assert!(
        state.status_line.contains("attached"),
        "Expected status line to mention 'attached', got: {}",
        state.status_line
    );

    let buf = super::common::render_buffer(&state, 80, 24);
    super::common::assert_buffer_contains(&buf, "Vault 'test-vault' attached");
}
