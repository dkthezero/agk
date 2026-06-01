//! Composition root: intentionally imports from `infra` to wire concrete adapters
//! into the Registry. This is the one permitted place where `app` depends on `infra`
//! (the "main" side of the hexagonal architecture). All other `app` code must not
//! import from `infra`.

pub use registry::build;
pub use scan::{build_vaults, filter_scan, scan, ScanError, ScanResult};
pub use state::{
    build_profile_entries, build_provider_entries, build_tab_kinds, build_vault_entries,
};

mod registry;
mod scan;
mod state;
