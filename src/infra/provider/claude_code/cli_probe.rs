use crate::app::ports::claude_cli_probe::{ClaudeCliProbePort, MIN_CLAUDE_CLI_VERSION};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub struct SystemClaudeCliProbe {
    path_override: Option<PathBuf>,
}

impl SystemClaudeCliProbe {
    pub fn new() -> Self {
        Self {
            path_override: None,
        }
    }
    /// Test-only constructor: temporarily override `$PATH` to a directory that
    /// does not contain `claude`, to force the unavailable path.
    pub fn with_path_override(path: &str) -> Self {
        Self {
            path_override: Some(PathBuf::from(path)),
        }
    }
}

impl Default for SystemClaudeCliProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeCliProbePort for SystemClaudeCliProbe {
    fn is_available(&self) -> bool {
        self.locate().is_ok()
    }

    fn locate(&self) -> Result<PathBuf> {
        let exe = "claude";
        let path = if let Some(p) = &self.path_override {
            // Test-only: search ONLY the override path so a `claude` binary
            // elsewhere on `$PATH` is ignored. This forces the unavailable path.
            if p.join(exe).is_file() {
                p.join(exe)
            } else {
                anyhow::bail!("claude not found on PATH");
            }
        } else {
            which::which(exe).with_context(|| "claude not found on PATH")?
        };
        Ok(path)
    }

    fn version(&self) -> Result<semver::Version> {
        let path = self.locate()?;
        let out = Command::new(&path)
            .arg("--version")
            .output()
            .with_context(|| format!("failed to run {} --version", path.display()))?;
        if !out.status.success() {
            anyhow::bail!("claude --version exited with {:?}", out.status.code());
        }
        let s = String::from_utf8_lossy(&out.stdout);
        let token = s
            .split_whitespace()
            .find(|t| t.chars().next().map_or(false, |c| c.is_ascii_digit()))
            .ok_or_else(|| anyhow::anyhow!("could not parse version from: {}", s.trim()))?;
        semver::Version::parse(token.trim_start_matches('v'))
            .with_context(|| format!("invalid semver: {token}"))
    }

    fn supports_agent_flag(&self) -> bool {
        self.version()
            .map_or(false, |v| v >= MIN_CLAUDE_CLI_VERSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::claude_cli_probe::ClaudeCliProbePort;

    #[test]
    fn system_probe_reports_unavailable_when_cli_missing() {
        // Force a PATH that definitely does not contain `claude`.
        let probe = SystemClaudeCliProbe::with_path_override("/this/path/does/not/exist");
        assert!(!probe.is_available());
        assert!(!probe.supports_agent_flag());
    }
}
