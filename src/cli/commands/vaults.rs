use super::*;
use crate::cli::entry::Cli;
use anyhow::Result;

// Vault attach/detach commands are wired to AgkCore.
// Full wiring lives in cli::core_dispatcher.
pub fn dispatch_vaults(_cli: &Cli, _workspace: &std::path::Path) -> Result<i32> {
    println!("Vault commands are wired to AgkCore; run via `cli::core_dispatcher`.");
    Ok(EXIT_SUCCESS)
}
