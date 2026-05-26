use super::*;
use crate::cli::entry::Cli;
use anyhow::Result;

// Apply/context commands are wired to AgkCore.
// Full wiring lives in cli::core_dispatcher.
pub fn dispatch_apply(_cli: &Cli, _workspace: &std::path::Path) -> Result<i32> {
    println!("Apply command is wired to AgkCore; run via `cli::core_dispatcher` instead.");
    Ok(EXIT_SUCCESS)
}
