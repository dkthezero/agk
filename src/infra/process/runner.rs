use crate::app::ports::ProcessRunnerPort;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Concrete [`ProcessRunnerPort`] using `std::process::Command`.
pub struct OsProcessRunner;

impl ProcessRunnerPort for OsProcessRunner {
    fn run(
        &self,
        command: &str,
        args: &[&str],
        cwd: Option<&Path>,
        env: Option<&[(String, String)]>,
    ) -> Result<String> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        if let Some(vars) = env {
            for (k, v) in vars {
                cmd.env(k, v);
            }
        }
        let output = cmd.output().with_context(|| format!("Failed to spawn {}", command))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Command '{}' exited with {}: {}",
                command,
                output.status.code().unwrap_or(-1),
                stderr.trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}
