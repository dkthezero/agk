//! Probes the host system for the `claude` CLI binary. Feature-gated by
//! `claude-cli-probe` at the use-case level (the port itself is always
//! available; only the real `SystemClaudeCliProbe` impl is gated).

use anyhow::Result;
use std::path::PathBuf;

/// Minimum version of the `claude` CLI that supports the `--agent` flag
/// required by the Claude Code provider. Older versions are rejected.
pub const MIN_CLAUDE_CLI_VERSION: semver::Version = semver::Version::new(2, 0, 0);

pub trait ClaudeCliProbePort: Send + Sync {
    /// `true` if the `claude` binary is on `$PATH` and runnable.
    fn is_available(&self) -> bool;
    /// Absolute path to the `claude` binary. Errors if not present.
    fn locate(&self) -> Result<PathBuf>;
    /// Parsed semver version (output of `claude --version`). Errors if the
    /// binary is missing or its output is unparseable.
    fn version(&self) -> Result<semver::Version>;
    /// `true` if the installed version is `>= MIN_CLAUDE_CLI_VERSION` and
    /// therefore supports the `--agent` flag.
    fn supports_agent_flag(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn system_probe_does_not_panic_when_cli_missing() {
        // Just exercise the trait; a real test of the system impl is in
        // infra::provider::claude_code::cli_probe.
        let p: Box<dyn ClaudeCliProbePort> = Box::new(MissingCliProbe);
        assert!(!p.is_available());
        assert!(p.locate().is_err());
    }

    struct MissingCliProbe;
    impl ClaudeCliProbePort for MissingCliProbe {
        fn is_available(&self) -> bool { false }
        fn locate(&self) -> Result<PathBuf> { anyhow::bail!("claude not on PATH") }
        fn version(&self) -> Result<semver::Version> { anyhow::bail!("claude not on PATH") }
        fn supports_agent_flag(&self) -> bool { false }
    }
}
