use crate::app::ports::ProcessRunnerPort;
use anyhow::{Context, Result};
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::Duration;

/// Concrete [`ProcessRunnerPort`] using `std::process::Command`.
pub struct OsProcessRunner;

impl ProcessRunnerPort for OsProcessRunner {
    #[cfg_attr(
        feature = "observability",
        tracing::instrument(skip(self), fields(command = %command))
    )]
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
        let output = cmd
            .output()
            .with_context(|| format!("Failed to spawn {}", command))?;
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

    #[cfg_attr(
        feature = "observability",
        tracing::instrument(skip(self), fields(command = %command))
    )]
    fn run_interactive(&self, command: &str, args: &[String], cwd: &Path) -> Result<ExitStatus> {
        Command::new(command)
            .current_dir(cwd)
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| format!("Failed to spawn interactive process {}", command))
    }

    #[cfg_attr(
        feature = "observability",
        tracing::instrument(skip(self), fields(command = %command, ?timeout))
    )]
    fn run_with_timeout(
        &self,
        command: &str,
        args: &[&str],
        cwd: Option<&Path>,
        env: Option<&[(String, String)]>,
        timeout: Duration,
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
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn {}", command))?;

        let _pid = child.id();
        let result = Self::wait_with_timeout(&mut child, timeout, command);

        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }

        let output = result?;
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

impl OsProcessRunner {
    fn wait_with_timeout(child: &mut Child, timeout: Duration, command: &str) -> Result<Output> {
        let start = std::time::Instant::now();
        let pid = child.id();
        loop {
            match child.try_wait()? {
                Some(status) => {
                    let mut out = Output {
                        status,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    };
                    if let Some(ref mut out_pipe) = child.stdout {
                        out_pipe.read_to_end(&mut out.stdout).ok();
                    }
                    if let Some(ref mut err_pipe) = child.stderr {
                        err_pipe.read_to_end(&mut out.stderr).ok();
                    }
                    return Ok(out);
                }
                None => {
                    if start.elapsed() >= timeout {
                        anyhow::bail!(
                            "Command '{}' (pid {}) timed out after {:?}",
                            command,
                            pid,
                            timeout
                        );
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
}
