use agk::app::command::CoreCommand;
use agk::app::features::mcp::command::RegisterMcpInput;
use agk::domain::mcp::McpTransport;
use agk::domain::scope::Scope;
use agk::tui::app::AppState;
use std::collections::HashMap;

/// Register an MCP server, then list it — verify the round-trip succeeds
/// and the status line reflects each step.
#[test]
fn mcp_register_and_list_roundtrip() {
    let core = super::common::test_core();
    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());

    // 1. Register an MCP server
    let register_cmd = CoreCommand::RegisterMcp {
        input: RegisterMcpInput {
            name: "test-mcp".into(),
            command: "echo".into(),
            args: vec![],
            env: vec![],
            transport: McpTransport::Stdio,
            description: Some("Test MCP server".into()),
            test_after: false,
        },
    };
    super::common::execute(&core, &mut state, register_cmd).unwrap();
    assert!(
        state.status_line.contains("registered"),
        "Expected 'registered' in status line after RegisterMcp, got: {}",
        state.status_line
    );

    // 2. List MCP servers — should show our registered server
    let list_cmd = CoreCommand::ListMcp;
    super::common::execute(&core, &mut state, list_cmd).unwrap();
    assert!(
        state.status_line.contains("listed"),
        "Expected 'listed' in status line after ListMcp, got: {}",
        state.status_line
    );
}

/// Register, enable, then disable an MCP server — verify each state transition
/// produces the correct event reflected in the status line.
#[test]
fn mcp_enable_disable_roundtrip() {
    let core = super::common::test_core();
    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());

    // 1. Register
    let register_cmd = CoreCommand::RegisterMcp {
        input: RegisterMcpInput {
            name: "test-mcp".into(),
            command: "echo".into(),
            args: vec![],
            env: vec![],
            transport: McpTransport::Stdio,
            description: None,
            test_after: false,
        },
    };
    super::common::execute(&core, &mut state, register_cmd).unwrap();
    assert!(
        state.status_line.contains("registered"),
        "Expected 'registered' after RegisterMcp, got: {}",
        state.status_line
    );

    // 2. Enable
    let enable_cmd = CoreCommand::EnableMcp {
        name: "test-mcp".into(),
        provider_id: "opencode".into(),
        scope: Scope::Workspace,
    };
    super::common::execute(&core, &mut state, enable_cmd).unwrap();
    assert!(
        state.status_line.contains("enabled"),
        "Expected 'enabled' after EnableMcp, got: {}",
        state.status_line
    );

    // 3. Disable
    let disable_cmd = CoreCommand::DisableMcp {
        name: "test-mcp".into(),
        provider_id: "opencode".into(),
        scope: Scope::Workspace,
    };
    super::common::execute(&core, &mut state, disable_cmd).unwrap();
    assert!(
        state.status_line.contains("disabled"),
        "Expected 'disabled' after DisableMcp, got: {}",
        state.status_line
    );

    // 4. Render final state — the disabled message should appear in the buffer
    let buf = super::common::render_buffer(&state, 80, 24);
    super::common::assert_buffer_contains(&buf, &state.status_line);
}

/// Verify that registering an MCP server and then rendering the TUI shows
/// the registered server name in the buffer.
#[test]
fn mcp_register_renders_in_tui() {
    let core = super::common::test_core();
    let mut state = AppState::new(
        vec!["Skills".into(), "MCP Servers".into()],
        vec![true, true],
        HashMap::new(),
    );

    let register_cmd = CoreCommand::RegisterMcp {
        input: RegisterMcpInput {
            name: "my-server".into(),
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
    super::common::execute(&core, &mut state, register_cmd).unwrap();

    // The status line should mention the registration
    assert!(
        state.status_line.contains("my-server"),
        "Expected 'my-server' in status line, got: {}",
        state.status_line
    );

    let buf = super::common::render_buffer(&state, 80, 24);
    super::common::assert_buffer_contains(&buf, "MCP 'my-server' registered");
}
