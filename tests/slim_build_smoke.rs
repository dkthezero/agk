//! Slim-build regression test.
//!
//! The "slim" runtime is the Docker slim image: it ships **only** the
//! minimum baseline features. Specifically it MUST NOT pull in the LLM
//! adapters (Ollama, LM Studio, Anthropic, OpenAI), the `profile-create`
//! YAML escape logic, or the `claude-cli-probe` shell-out used at profile
//! creation time. The TUI feature is the user-facing default and is kept.
//!
//! Concretely the slim feature set is `tui` only, built with
//! `--no-default-features --features tui`. This test is the guard rail
//! that keeps that promise: it must (a) compile cleanly with zero
//! warnings, and (b) produce a binary that responds to `--version` and
//! `--help`. If a feature-gated import sneaks into a non-gated module
//! the slim build will fail, and so will this test.
//!
//! This is intentionally a shell-driven test rather than a `build.rs`
//! trick — invoking `cargo` from inside a test exercises the real build
//! pipeline end-to-end and would catch a feature-gated regression that
//! a build script cannot.

use std::process::Command;

const SLIM_FEATURES: &str = "tui";

/// Resolve the path to the `agk` binary that cargo will build for
/// integration tests in this package.
///
/// Integration tests in `tests/*.rs` get `CARGO_BIN_EXE_<bin>` set by
/// cargo when the corresponding `[[bin]]` is built first. As a fallback
/// (e.g. when running the test binary directly) we look next to
/// `target/debug/agk` under the manifest directory.
fn agk_bin_path() -> String {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_agk") {
        return p;
    }
    format!("{}/target/debug/agk", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn slim_build_compiles_and_runs() {
    // Step 1: build the slim binary. Treat warnings as errors so the
    // regression test is strict about unused / dead-code imports that
    // a no-default-features build might surface.
    let build = Command::new("cargo")
        .args([
            "build",
            "--no-default-features",
            "--features",
            SLIM_FEATURES,
            "--bin",
            "agk",
        ])
        .env("RUSTFLAGS", "-D warnings")
        .output()
        .expect("failed to invoke cargo build");

    assert!(
        build.status.success(),
        "slim build failed:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr),
    );

    let bin = agk_bin_path();

    // Step 2: `--version` must succeed and emit a non-empty version line.
    let version = Command::new(&bin)
        .arg("--version")
        .output()
        .expect("failed to run agk --version");

    assert!(
        version.status.success(),
        "agk --version exited non-zero: {:?}\nstderr: {}",
        version.status.code(),
        String::from_utf8_lossy(&version.stderr),
    );
    let version_str = String::from_utf8_lossy(&version.stdout);
    assert!(
        !version_str.trim().is_empty(),
        "agk --version produced empty output",
    );
    // Clap puts the binary name in front of the version; sanity check it
    // is actually the agk binary.
    assert!(
        version_str.to_ascii_lowercase().contains("agk"),
        "--version output did not mention 'agk': {:?}",
        version_str,
    );

    // Step 3: `--help` must succeed and produce a usage banner.
    let help = Command::new(&bin)
        .arg("--help")
        .output()
        .expect("failed to run agk --help");

    assert!(
        help.status.success(),
        "agk --help exited non-zero: {:?}\nstderr: {}",
        help.status.code(),
        String::from_utf8_lossy(&help.stderr),
    );
    let help_str = String::from_utf8_lossy(&help.stdout);
    assert!(
        help_str.contains("Usage") || help_str.to_ascii_lowercase().contains("agk"),
        "agk --help output looks wrong: {:?}",
        help_str,
    );
}
