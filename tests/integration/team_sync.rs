//! Integration tests for `agk team init` and `agk vault init`.
//!
//! These tests exercise the compiled binary end-to-end via `assert_cmd`,
//! verifying that the CLI creates the expected files and directories on disk.

use assert_cmd::Command;

#[test]
fn team_init_creates_team_toml() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("agk").unwrap();
    cmd.args(["team", "init", "--name", "test-team"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(dir.path().join(".agk/team.toml").exists());
    assert!(dir.path().join(".agk/.gitignore").exists());
}

#[test]
fn vault_init_creates_vault_toml_and_folders() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("agk").unwrap();
    cmd.args(["vault", "init", "--name", "my-vault"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(dir.path().join(".agk/vault.toml").exists());
    assert!(dir.path().join("skills").exists());
    assert!(dir.path().join("instructions").exists());
    assert!(dir.path().join("mcps").exists());
    assert!(dir.path().join("profiles").exists());
}
