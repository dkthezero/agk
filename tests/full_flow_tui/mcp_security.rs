//! Integration tests for MCP Security Scorecard (F20).
//!
//! Tests that:
//! - assess_mcp_security returns correct flags for known patterns
//! - SecurityFlag severity and badge methods work correctly
//! - McpServer with security_flags deserializes correctly
//! - Backward compatibility: McpServer without security_flags still works

use agk::domain::mcp::{McpActivation, McpServer, McpTransport};
use agk::domain::mcp_security::{assess_mcp_security, SecurityFlag, SecuritySeverity};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Heuristic tests
// ---------------------------------------------------------------------------

#[test]
fn broad_filesystem_root_path() {
    let flags = assess_mcp_security("npx", &["-y".into(), "@modelcontextprotocol/server-filesystem".into(), "/".into()]);
    assert!(flags.contains(&SecurityFlag::BroadFilesystem));
}

#[test]
fn broad_filesystem_home_dir() {
    let flags = assess_mcp_security("npx", &["-y".into(), "some-server".into(), "~".into()]);
    assert!(flags.contains(&SecurityFlag::BroadFilesystem));
}

#[test]
fn broad_filesystem_cwd() {
    let flags = assess_mcp_security("npx", &["-y".into(), "some-server".into(), ".".into()]);
    assert!(flags.contains(&SecurityFlag::BroadFilesystem));
}

#[test]
fn network_egress_http() {
    let flags = assess_mcp_security("node", &["server.js".into(), "http://example.com".into()]);
    assert!(flags.contains(&SecurityFlag::NetworkEgress));
}

#[test]
fn network_egress_curl() {
    let flags = assess_mcp_security("curl", &["https://example.com".into()]);
    assert!(flags.contains(&SecurityFlag::NetworkEgress));
}

#[test]
fn network_egress_wget() {
    let flags = assess_mcp_security("wget", &["https://example.com".into()]);
    assert!(flags.contains(&SecurityFlag::NetworkEgress));
}

#[test]
fn arbitrary_execution_bash_script() {
    let flags = assess_mcp_security("bash", &["script.sh".into()]);
    assert!(flags.contains(&SecurityFlag::ArbitraryExecution));
}

#[test]
fn arbitrary_execution_python_script() {
    let flags = assess_mcp_security("python", &["script.py".into()]);
    assert!(flags.contains(&SecurityFlag::ArbitraryExecution));
}

#[test]
fn arbitrary_execution_python3_script() {
    let flags = assess_mcp_security("python3", &["exploit.py".into()]);
    assert!(flags.contains(&SecurityFlag::ArbitraryExecution));
}

#[test]
fn arbitrary_execution_bash_no_script() {
    // bash without .sh or .py args should NOT trigger arbitrary execution
    let flags = assess_mcp_security("bash", &["-c".into(), "echo hello".into()]);
    assert!(!flags.contains(&SecurityFlag::ArbitraryExecution));
}

#[test]
fn env_exfiltration_home() {
    let flags = assess_mcp_security("npx", &["-y".into(), "some-server".into(), "$HOME/path".into()]);
    assert!(flags.contains(&SecurityFlag::EnvExfiltration));
}

#[test]
fn env_exfiltration_ssh_key() {
    let flags = assess_mcp_security("npx", &["-y".into(), "some-server".into(), "$SSH_KEY".into()]);
    assert!(flags.contains(&SecurityFlag::EnvExfiltration));
}

#[test]
fn unspecified_args_no_args() {
    let flags = assess_mcp_security("node", &[]);
    assert!(flags.contains(&SecurityFlag::UnspecifiedArgs));
}

#[test]
fn safe_mcp_no_flags() {
    let flags = assess_mcp_security("npx", &["-y".into(), "@modelcontextprotocol/server-filesystem".into()]);
    // This has no broad-filesystem, no network, no env exfil, has args
    assert!(!flags.contains(&SecurityFlag::BroadFilesystem));
    assert!(!flags.contains(&SecurityFlag::NetworkEgress));
    assert!(!flags.contains(&SecurityFlag::ArbitraryExecution));
    assert!(!flags.contains(&SecurityFlag::EnvExfiltration));
    assert!(!flags.contains(&SecurityFlag::UnspecifiedArgs));
}

#[test]
fn multiple_flags_simultaneously() {
    let flags = assess_mcp_security("bash", &["script.sh".into(), "$HOME/data".into()]);
    assert!(flags.contains(&SecurityFlag::ArbitraryExecution));
    assert!(flags.contains(&SecurityFlag::EnvExfiltration));
    // BroadFilesystem only matches exact "/", "~", or "." args, not paths containing them
}

// ---------------------------------------------------------------------------
// Severity and badge tests
// ---------------------------------------------------------------------------

#[test]
fn severity_mapping() {
    assert_eq!(SecurityFlag::UnspecifiedArgs.severity(), SecuritySeverity::Low);
    assert_eq!(SecurityFlag::EnvExfiltration.severity(), SecuritySeverity::Medium);
    assert_eq!(SecurityFlag::BroadFilesystem.severity(), SecuritySeverity::High);
    assert_eq!(SecurityFlag::NetworkEgress.severity(), SecuritySeverity::High);
    assert_eq!(SecurityFlag::ArbitraryExecution.severity(), SecuritySeverity::Critical);
}

#[test]
fn badge_display() {
    assert_eq!(SecurityFlag::UnspecifiedArgs.badge(), "[i]");
    assert_eq!(SecurityFlag::EnvExfiltration.badge(), "[!]");
    assert_eq!(SecurityFlag::BroadFilesystem.badge(), "[!]");
    assert_eq!(SecurityFlag::NetworkEgress.badge(), "[!]");
    assert_eq!(SecurityFlag::ArbitraryExecution.badge(), "[!!]");
}

#[test]
fn description_text() {
    assert!(!SecurityFlag::BroadFilesystem.description().is_empty());
    assert!(!SecurityFlag::ArbitraryExecution.description().is_empty());
}

// ---------------------------------------------------------------------------
// McpServer backward compatibility
// ---------------------------------------------------------------------------

#[test]
fn mcp_server_with_security_flags_roundtrip() {
    let mut activation = HashMap::new();
    activation.insert("claude-code".to_string(), McpActivation {
        global: true,
        workspace: false,
    });

    let server = McpServer {
        name: "filesystem".to_string(),
        command: "npx".to_string(),
        args: vec!["-y".into(), "@modelcontextprotocol/server-filesystem".into(), "/".into()],
        env: HashMap::new(),
        transport: McpTransport::Stdio,
        description: Some("File system access".to_string()),
        tested: true,
        tested_at: Some("2026-06-01T00:00:00Z".to_string()),
        activation,
        security_flags: vec![SecurityFlag::BroadFilesystem],
    };

    let json = serde_json::to_string(&server).unwrap();
    let parsed: McpServer = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.security_flags.len(), 1);
    assert_eq!(parsed.security_flags[0], SecurityFlag::BroadFilesystem);
}

#[test]
fn mcp_server_without_security_flags_deserializes() {
    // Old JSON without security_flags should deserialize with empty vec
    let json = r#"{
        "name": "safe-server",
        "command": "node",
        "args": ["server.js"],
        "env": {},
        "transport": "stdio",
        "description": null,
        "tested": false,
        "tested_at": null,
        "activation": {}
    }"#;

    let parsed: McpServer = serde_json::from_str(json).unwrap();
    assert!(parsed.security_flags.is_empty());
    assert_eq!(parsed.name, "safe-server");
}