//! Port trait for ClawHub CLI operations.
//!
//! Introduced to eliminate ADR-001 violations where `app/features/` called
//! `infra::vault::clawhub` free functions directly. All app-layer code should
//! go through this port; only the infra adapter implementation may call the
//! underlying free functions.

use anyhow::Result;

/// Abstraction over ClawHub CLI operations.
///
/// The default production adapter delegates to the free functions in
/// `infra::vault::clawhub`. Test code injects `FakeClawHub` instead.
pub trait ClawHubPort: Send + Sync {
    /// Check if the `clawhub` CLI is available on `$PATH`.
    fn is_cli_available(&self) -> bool;

    /// Check if Homebrew is available (macOS).
    fn is_homebrew_available(&self) -> bool;

    /// Install the ClawHub CLI via Homebrew.
    fn install_cli(&self) -> Result<()>;

    /// Run `clawhub install <slug>` to fetch a remote asset.
    ///
    /// Accepts `"owner/slug"` format — the adapter extracts the slug.
    fn cli_install(&self, identity: &str) -> Result<()>;
}
