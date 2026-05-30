//! Contract Tests: CLI --dry-run --json output must adhere to fixed shape.
//!
//! These tests verify that golden fixtures are valid JSON and that the
//! expected contract schema is present in source.  Full end-to-end contract
//! testing requires `cargo build` + a populated workspace profile.

use assert_cmd::prelude::CommandCargoExt;
use std::path::Path;

#[test]
fn golden_fixtures_are_valid_json() {
    let fixture_dir = Path::new("fixtures/contracts");
    if !fixture_dir.exists() {
        // Skip if fixtures haven't been generated yet
        return;
    }
    let entries: Vec<_> = std::fs::read_dir(fixture_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    assert!(
        !entries.is_empty(),
        "Expected at least one .json fixture under fixtures/contracts/"
    );
    for entry in entries {
        let text = std::fs::read_to_string(entry.path())
            .unwrap_or_else(|e| panic!("{}: {}", entry.path().display(), e));
        let json: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{} is invalid JSON: {}", entry.path().display(), e));
        assert!(
            json.get("profile_id").is_some() || json.get("provider_id").is_some(),
            "Fixture {} must contain profile_id or provider_id",
            entry.path().display()
        );
    }
}

#[test]
fn binary_can_be_invoked_with_help() {
    let mut cmd = std::process::Command::cargo_bin("agk").expect("cargo_bin not found");
    cmd.args(["--help"]);
    let output = cmd.output().expect("failed to spawn agk");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("agk") || output.status.success(),
        "Expected agk help output, got stdout={} stderr={}",
        stdout,
        stderr
    );
}

/// Contract: `agk debug tasks --json` must emit valid JSON with an `events` array.
#[test]
fn debug_tasks_json_schema() {
    let mut cmd = std::process::Command::cargo_bin("agk").expect("cargo_bin not found");
    cmd.args(["debug", "tasks", "--json"]);
    let output = cmd.output().expect("failed to spawn agk");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "debug tasks failed: stdout={} stderr={}",
        stdout,
        stderr
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "debug tasks --json produced invalid JSON: {}\n{}",
            e, stdout
        )
    });
    assert!(
        json.get("events").is_some(),
        "JSON output must contain top-level 'events' array"
    );
}

/// Contract: `agk debug hangs --json` must emit valid JSON with an `events` array.
#[test]
fn debug_hangs_json_schema() {
    let mut cmd = std::process::Command::cargo_bin("agk").expect("cargo_bin not found");
    cmd.args(["debug", "hangs", "--json"]);
    let output = cmd.output().expect("failed to spawn agk");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "debug hangs failed: stdout={} stderr={}",
        stdout,
        stderr
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "debug hangs --json produced invalid JSON: {}\n{}",
            e, stdout
        )
    });
    assert!(
        json.get("events").is_some(),
        "JSON output must contain top-level 'events' array"
    );
}

/// Contract: `agk mcp list --json` must emit only valid JSON objects
/// (either NDJSON streamed events or a final summary).
#[test]
fn mcp_list_json_schema() {
    let mut cmd = std::process::Command::cargo_bin("agk").expect("cargo_bin not found");
    cmd.args(["mcp", "list", "--json"]);
    let output = cmd.output().expect("failed to spawn agk");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "mcp list failed: stdout={} stderr={}",
        stdout,
        stderr
    );

    let mut found_events_summary = false;
    let stream = serde_json::Deserializer::from_str(&stdout).into_iter::<serde_json::Value>();
    for result in stream {
        let json = result
            .unwrap_or_else(|e| panic!("mcp list --json produced invalid JSON: {}\n{}", e, stdout));
        if json.get("events").is_some() {
            found_events_summary = true;
        }
    }
    assert!(
        found_events_summary,
        "JSON output must contain at least one top-level 'events' array"
    );
}
