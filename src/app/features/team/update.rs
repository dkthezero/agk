use anyhow::Result;
use std::path::PathBuf;

/// Update team.toml from the source repository.
///
/// TODO(team-p2): Implement git-pull logic for team.toml updates.
/// For now, users must update team.toml manually or via `team add/remove`.
///
/// Returns `Err` because the operation is not implemented: callers (CLI/TUI)
/// must see a non-success result so scripts checking `$?` don't mistake the
/// no-op for a successful update. The helpful "not yet implemented" message
/// is carried by the `Err` and surfaced to the user via the dispatcher's
/// error-rendering path (text: `Error: ...`, JSON: structured `Error` event).
pub fn team_update(_workspace_root: &PathBuf) -> Result<()> {
    Err(anyhow::anyhow!(
        "Team update is not yet implemented. Pull team.toml changes manually for now."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_returns_not_yet_implemented_error() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let result = team_update(&workspace);
        let err = result.expect_err("unimplemented team_update must return Err");
        assert!(
            err.to_string().contains("not yet implemented"),
            "error message should explain the unimplemented state: {err}"
        );
    }
}
