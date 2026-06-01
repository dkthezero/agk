//! Fake ClawHub port for testing.

use crate::app::ports::ClawHubPort;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Fake ClawHub port for testing.
///
/// By default `is_cli_available()` returns `true` (so attach never tries to
/// install) and all other operations succeed. Toggle behaviour via the public
/// fields.
pub struct FakeClawHub {
    /// Whether `is_cli_available()` returns true. Defaults to `true`.
    pub cli_available: AtomicBool,
    /// Whether `is_homebrew_available()` returns true. Defaults to `true`.
    pub homebrew_available: AtomicBool,
    /// Whether `install_cli()` succeeds. Defaults to `true`.
    pub install_succeeds: AtomicBool,
    /// Whether `cli_install()` succeeds. Defaults to `true`.
    pub cli_install_succeeds: AtomicBool,
    /// Identities that were passed to `cli_install()`.
    pub installed: Mutex<Vec<String>>,
}

impl FakeClawHub {
    pub fn new() -> Self {
        Self {
            cli_available: AtomicBool::new(true),
            homebrew_available: AtomicBool::new(true),
            install_succeeds: AtomicBool::new(true),
            cli_install_succeeds: AtomicBool::new(true),
            installed: Mutex::new(Vec::new()),
        }
    }
}

impl Default for FakeClawHub {
    fn default() -> Self {
        Self::new()
    }
}

impl ClawHubPort for FakeClawHub {
    fn is_cli_available(&self) -> bool {
        self.cli_available.load(Ordering::SeqCst)
    }

    fn is_homebrew_available(&self) -> bool {
        self.homebrew_available.load(Ordering::SeqCst)
    }

    fn install_cli(&self) -> anyhow::Result<()> {
        if self.install_succeeds.load(Ordering::SeqCst) {
            Ok(())
        } else {
            anyhow::bail!("FakeClawHub: install_cli failed")
        }
    }

    fn cli_install(&self, identity: &str) -> anyhow::Result<()> {
        self.installed.lock().unwrap().push(identity.to_string());
        if self.cli_install_succeeds.load(Ordering::SeqCst) {
            Ok(())
        } else {
            anyhow::bail!("FakeClawHub: cli_install failed for '{}'", identity)
        }
    }
}
