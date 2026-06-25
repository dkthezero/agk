//! `agk team` subcommand enum.
//!
//! Split out of `entry_subcommands.rs` to satisfy the ADR-001 §6.4
//! file-size limit. Re-exported by `entry.rs` so consumer paths
//! (`crate::cli::entry::TeamCommands`) remain unchanged.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum TeamCommands {
    /// Initialize team configuration
    Init {
        /// Team name
        #[arg(short, long)]
        name: String,
        /// Only show what would change
        #[arg(long)]
        dry_run: bool,
    },
    /// Add a vault to the team marketplace
    AddVault {
        /// Vault identity
        identity: String,
        /// Vault type (github, local, clawhub)
        #[arg(long, name = "type", default_value = "github")]
        vault_type: String,
        /// Vault URL
        #[arg(long)]
        url: String,
        /// Branch
        #[arg(short, long, default_value = "main")]
        branch: String,
    },
    /// Add a skill requirement to the team
    Add {
        /// Skill identity (e.g., acme-org/react-conventions)
        identity: String,
        /// Vault to install from
        #[arg(long)]
        vault: String,
        /// Asset kind
        #[arg(long, default_value = "skill")]
        kind: String,
        /// Version constraint (e.g., >= 2.0.0)
        #[arg(long)]
        version_constraint: Option<String>,
    },
    /// Remove a skill requirement from the team
    Remove {
        /// Skill identity to remove
        identity: String,
    },
    /// Show diff between team requirements and installed state
    Diff,
    /// Show team status (installed vs missing)
    Status,
    /// Update team.toml from source repository (not yet implemented)
    Update,
}
