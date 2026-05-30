//! Process Integration Tests (P8 Layer 3).
//!
//! Exercises the real [`OsProcessRunner`] against short-lived shell commands
//! to assert stdout capture, exit-code handling, and timeout enforcement.

use agk::app::ports::ProcessRunnerPort;
use agk::infra::process::runner::OsProcessRunner;
use std::time::Duration;

#[test]
fn runner_captures_stdout() {
    let runner = OsProcessRunner;
    let out = runner.run("echo", &["hello"], None, None).unwrap();
    assert_eq!(out, "hello");
}

#[test]
fn runner_fails_on_bad_exit() {
    let runner = OsProcessRunner;
    let result = runner.run("false", &[], None, None);
    assert!(result.is_err(), "Expected 'false' to return an error");
}

#[test]
fn runner_captures_stderr_in_error() {
    let runner = OsProcessRunner;
    let result = runner.run("bash", &["-c", "echo error-msg >&2; exit 1"], None, None);
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("error-msg"),
        "Expected stderr to appear in error message, got: {}",
        err
    );
}

#[test]
fn runner_with_timeout_honours_deadline() {
    let runner = OsProcessRunner;
    let result = runner.run_with_timeout("sleep", &["10"], None, None, Duration::from_millis(100));
    assert!(result.is_err(), "Expected timeout error for sleep 10");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("timed out"),
        "Expected 'timed out' in error, got: {}",
        err
    );
}

#[test]
fn runner_interactive_returns_exit_status() {
    let runner = OsProcessRunner;
    let dir = std::env::temp_dir();
    let status = runner.run_interactive("true", &[], &dir).unwrap();
    assert!(status.success());
}
