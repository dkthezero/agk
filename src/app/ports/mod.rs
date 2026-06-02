//! Port traits — the application's boundary contracts.
//!
//! One trait per file, grouped by capability. All public traits and types are
//! re-exported from this module so callers can continue to write
//! `use crate::app::ports::FooPort` without caring how the file is split.

pub mod clawhub;
pub mod config_store;
pub mod context_store;
pub mod feature_set;
pub mod file_opener;
pub mod manifest_codec;
pub mod mcp_registry;
pub mod process_runner;
pub mod profile_runtime;
pub mod provider;
pub mod task_tracker;
pub mod team_config_store;
pub mod telemetry_store;
pub mod vault;
pub mod vault_manifest_store;
pub mod wizard_state;

pub use clawhub::ClawHubPort;
pub use config_store::ConfigStorePort;
pub use context_store::ContextStorePort;
pub use feature_set::FeatureSetPort;
pub use file_opener::FileOpenerPort;
pub use manifest_codec::ManifestCodecPort;
pub use mcp_registry::{McpProvider, McpRegistryPort};
pub use process_runner::ProcessRunnerPort;
pub use profile_runtime::{ProfileRuntimePort, ProfileSession};
pub use provider::{ArchetypeTemplate, ProviderPort, WizardStep};
pub use task_tracker::{TaskPhase, TaskTrackerPort, TrackedTask};
pub use team_config_store::TeamConfigStorePort;
pub use telemetry_store::TelemetryStorePort;
pub use vault::{VaultPort, VaultSearchPort};
pub use vault_manifest_store::VaultManifestStorePort;
pub use wizard_state::WizardState;
