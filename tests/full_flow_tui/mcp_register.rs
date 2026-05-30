use agk::app::command::CoreCommand;
use agk::app::features::mcp::command::RegisterMcpInput;
use agk::domain::mcp::McpTransport;
use agk::tui::app::AppState;
use std::collections::HashMap;

#[test]
fn mcp_register_updates_status_line() {
    let core = super::common::test_core();
    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());

    let cmd = CoreCommand::RegisterMcp {
        input: RegisterMcpInput {
            name: "fs-server".into(),
            command: "npx".into(),
            args: vec![
                "-y".into(),
                "@modelcontextprotocol/server-filesystem".into(),
            ],
            env: vec![],
            transport: McpTransport::Stdio,
            description: Some("Filesystem MCP".into()),
            test_after: false,
        },
    };

    super::common::execute(&core, &mut state, cmd).unwrap();
    assert!(
        state.status_line.contains("registered"),
        "Expected 'registered' in status line, got: {}",
        state.status_line
    );

    let buf = super::common::render_buffer(&state, 80, 24);
    super::common::assert_buffer_contains(&buf, "MCP 'fs-server' registered");
}
