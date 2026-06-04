//! Golden contract tests for `render_agent_markdown`.
//!
//! These tests lock in the exact YAML/markdown output of the renderer for
//! representative inputs. If the Claude Code frontmatter format ever changes
//! upstream, or our renderer deviates from the documented shape, the fixtures
//! catch it.
//!
//! Each fixture pair consists of:
//!   * `<case>.json`        — the `AgentFrontmatter` serialized as JSON,
//!                             deserialized into a `LaunchPlan` by the test.
//!   * `<case>.expected.md` — the verbatim string the renderer must produce
//!                             for that plan (compared via `assert_eq!` after
//!                             trimming trailing whitespace from both sides).
//!
//! The `LaunchPlan` is built per-test from the deserialized frontmatter plus
//! a `resolved_mcp_servers` list (full case) or empty (basic / minimal).
//! `prompt_body` is taken from the fixture where the case includes a body
//! (full) and is empty for the basic / minimal cases.

use std::path::PathBuf;

use agk::domain::agent_markdown::{AgentFrontmatter, AgentMcpServer};
use agk::domain::launch_plan::LaunchPlan;
use agk::infra::provider::claude_code::agent_markdown::render_agent_markdown;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/agent_markdown")
}

fn load_frontmatter(case: &str) -> AgentFrontmatter {
    let path = fixture_dir().join(case);
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()));
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse fixture {}: {e}", path.display()))
}

fn load_expected(case: &str) -> String {
    let path = fixture_dir().join(case);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
}

fn assert_matches(case_json: &str, case_expected: &str, plan: LaunchPlan) {
    let rendered = render_agent_markdown(&plan);
    let expected = load_expected(case_expected);
    assert_eq!(
        rendered.trim_end(),
        expected.trim_end(),
        "renderer output diverged from golden fixture {case_expected}"
    );
    // Sanity: also confirm the JSON fixture itself is still valid.
    let _ = load_frontmatter(case_json);
}

#[test]
fn render_matches_basic_agent_fixture() {
    let fm = load_frontmatter("basic_agent.json");
    let plan = LaunchPlan {
        profile_id: "test-agent".into(),
        provider_id: "claude-code".into(),
        frontmatter: fm,
        prompt_body: String::new(),
        resolved_mcp_servers: vec![],
        llm_provider_id: None,
    };
    assert_matches("basic_agent.json", "basic_agent.expected.md", plan);
}

#[test]
fn render_matches_minimal_agent_fixture() {
    let fm = load_frontmatter("minimal_agent.json");
    let plan = LaunchPlan {
        profile_id: "minimal-agent".into(),
        provider_id: "claude-code".into(),
        frontmatter: fm,
        prompt_body: String::new(),
        resolved_mcp_servers: vec![],
        llm_provider_id: None,
    };
    assert_matches("minimal_agent.json", "minimal_agent.expected.md", plan);
}

#[test]
fn render_matches_full_agent_fixture() {
    let fm = load_frontmatter("full_agent.json");
    let resolved_mcp_servers = vec![
        AgentMcpServer {
            name: "github".into(),
            command: "docker".into(),
            args: vec!["run".into(), "-i".into(), "mcp/github".into()],
            env: vec!["GITHUB_TOKEN=ghp_example".into()],
        },
        AgentMcpServer {
            name: "playwright".into(),
            command: "npx".into(),
            args: vec!["-y".into(), "@playwright/mcp".into()],
            env: vec![],
        },
    ];
    let plan = LaunchPlan {
        profile_id: "full-agent".into(),
        provider_id: "claude-code".into(),
        frontmatter: fm,
        prompt_body: "You are the full agent. Use all 16 frontmatter fields.".into(),
        resolved_mcp_servers,
        llm_provider_id: Some("local-ollama".into()),
    };
    assert_matches("full_agent.json", "full_agent.expected.md", plan);
}
