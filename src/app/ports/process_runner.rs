use anyhow::Result;
use std::path::Path;
use std::process::ExitStatus;

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

    /// Run a command that inherits the parent's stdin/stdout/stderr — used
    /// when the child needs to take over the terminal (e.g. interactive
    /// agents). Blocks until the child exits and returns its `ExitStatus`.
    ///
    /// The default implementation bails: only adapters that genuinely support
    /// terminal-inheriting processes (e.g. `OsProcessRunner`) should override.
    fn run_interactive(&self, command: &str, args: &[String], cwd: &Path) -> Result<ExitStatus> {
        let _ = (command, args, cwd);
        anyhow::bail!("interactive process not supported by this runner")
    }
}
