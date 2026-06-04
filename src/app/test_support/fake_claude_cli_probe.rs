//! Fake [`ClaudeCliProbePort`] for testing.
//!
//! Hand-rolled in-memory fake (no mocking libraries) that simulates the
//! presence/absence of a `claude` binary on `$PATH`.

use crate::app::ports::claude_cli_probe::{ClaudeCliProbePort, MIN_CLAUDE_CLI_VERSION};
use anyhow::Result;
use std::path::PathBuf;

/// Fake `claude` CLI probe.
///
/// Defaults to "unavailable" so unit tests that don't care about the CLI
/// don't accidentally exercise the agent-flag code path. Use
/// [`FakeClaudeCliProbe::available`] to seed a specific version.
pub struct FakeClaudeCliProbe {
    /// Whether `is_available()` returns `true`.
    pub available: bool,
    /// Path returned from `locate()` when available.
    pub path: PathBuf,
    /// Parsed version returned from `version()` when available.
    pub version: Option<semver::Version>,
}

impl FakeClaudeCliProbe {
    /// Build a probe that reports the `claude` CLI as missing.
    pub fn unavailable() -> Self {
        Self {
            available: false,
            path: PathBuf::from("/nonexistent/claude"),
            version: None,
        }
    }

    /// Build a probe that reports the `claude` CLI as present at a fixed
    /// path, with the given semver version string.
    pub fn available(v: &str) -> Self {
        Self {
            available: true,
            path: PathBuf::from("/usr/local/bin/claude"),
            version: Some(
                semver::Version::parse(v).expect("FakeClaudeCliProbe::available: valid semver"),
            ),
        }
    }
}

impl ClaudeCliProbePort for FakeClaudeCliProbe {
    fn is_available(&self) -> bool {
        self.available
    }

    fn locate(&self) -> Result<PathBuf> {
        if self.available {
            Ok(self.path.clone())
        } else {
            anyhow::bail!("claude not on PATH")
        }
    }

    fn version(&self) -> Result<semver::Version> {
        self.version
            .clone()
            .ok_or_else(|| anyhow::anyhow!("claude not on PATH"))
    }

    fn supports_agent_flag(&self) -> bool {
        self.version
            .as_ref()
            .map_or(false, |v| v >= &MIN_CLAUDE_CLI_VERSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::claude_cli_probe::ClaudeCliProbePort;

    #[test]
    fn fake_reports_unavailable_by_default() {
        let f = FakeClaudeCliProbe::unavailable();
        assert!(!f.is_available());
        assert!(f.locate().is_err());
    }

    #[test]
    fn fake_supports_agent_flag_when_version_high_enough() {
        let f = FakeClaudeCliProbe::available("2.1.0");
        assert!(f.supports_agent_flag());
        let old = FakeClaudeCliProbe::available("1.9.0");
        assert!(!old.supports_agent_flag());
    }
}
