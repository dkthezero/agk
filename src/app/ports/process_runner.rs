use anyhow::Result;
use std::path::Path;

/// Port for spawning external processes.
///
/// Isolates `std::process::Command` in the infrastructure layer so the
/// application and domain layers remain pure.
pub trait ProcessRunnerPort: Send + Sync {
    /// Run a command with the given arguments in the specified working directory.
    /// Returns the process stdout on success, or an error on failure.
    fn run(
        &self,
        command: &str,
        args: &[&str],
        cwd: Option<&Path>,
        env: Option<&[(String, String)]>,
    ) -> Result<String>;
}
