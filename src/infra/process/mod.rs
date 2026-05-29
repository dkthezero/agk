//! Process and OS-shell adapters.
//!
//! `infra/process/` is the only crate location (besides `main.rs`) where
//! `std::process::Command` may appear. All other layers reach process
//! capabilities via the port traits in `app/ports.rs`.

pub mod opener;
pub mod runner;
