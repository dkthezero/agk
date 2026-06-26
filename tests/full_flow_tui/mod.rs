//! End-to-End Full-Flow TUI Tests (P8 Layer 1 — Terminal Emulation).
//!
//! Each scenario constructs a headless [`TestBackend`], runs a [`CoreCommand`]
//! through [`AgkCore`] with a sink that feeds events straight into
//! [`AppState`], draws the resulting frame, and asserts on the rendered text.
//!
//! This reproduces the actual user experience without a real terminal or
//! async event loop, giving us deterministic frame-level assertions.

mod auto_install_deps;
mod common;
mod ghes_vault;
mod mcp_register;
mod mcp_roundtrip;
mod mcp_security;
mod profile_export_import;
mod provider_root_install;
mod provider_toggle;
mod skill_install;
mod sync_update;
mod telemetry_extensions;
mod vault_attach;
mod vault_profile_install_start;
mod wizard_full_flow;
