//! Subcommand enums for the `agk` CLI.
//!
//! Extracted from `entry.rs` to satisfy the ADR-001 §6.4 file-size limit.
//! Re-exported by `entry.rs` so consumer paths
//! (`crate::cli::entry::ProfileCommands`, etc.) remain unchanged.

use crate::cli::entry::{ExportFormat, ScopeArg};
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ProfileCommands {
    /// Start (launch) a profile session
    Start {
        /// Profile name
        name: String,

        /// Only show the launch plan without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Create a new profile headlessly and generate the agent file
    Create {
        /// Profile name
        name: String,

        /// Provider to use (default: opencode)
        #[arg(short, long, default_value = "opencode")]
        provider: String,

        /// Comma-separated list of skill names to include
        #[arg(short = 'k', long, value_delimiter = ',')]
        skills: Vec<String>,

        /// Comma-separated list of MCP server names to enable
        #[arg(short = 'm', long, value_delimiter = ',')]
        mcps: Vec<String>,

        /// Agent description (can be a path to a markdown file, or raw text)
        #[arg(short, long)]
        description: Option<String>,

        /// Read description from a markdown file
        #[arg(long)]
        description_file: Option<String>,

        /// Scope for storing the profile config
        #[arg(short, long, value_enum, default_value = "workspace")]
        scope: ScopeArg,

        /// Only show what would change
        #[arg(long)]
        dry_run: bool,
    },

    /// Export a profile to a portable JSON file
    Export {
        /// Profile name
        name: String,

        /// Output file path
        #[arg(short, long)]
        file: String,

        /// Resolve "auto" vault refs to actual vault names
        #[arg(long)]
        resolve_vaults: bool,

        /// Target scope
        #[arg(short, long, value_enum, default_value = "workspace")]
        scope: ScopeArg,
    },

    /// Import a profile from a portable JSON file
    Import {
        /// Path to .agk.json file
        file_path: String,

        /// Override profile name
        #[arg(short, long)]
        name: Option<String>,

        /// Target scope
        #[arg(short, long, value_enum, default_value = "workspace")]
        scope: ScopeArg,
    },

    /// Show differences between a local profile and its vault source
    Diff {
        /// Profile name
        name: String,

        /// Target scope
        #[arg(short, long, value_enum, default_value = "workspace")]
        scope: ScopeArg,
    },
}

#[derive(Subcommand, Debug)]
pub enum McpCommands {
    /// Add/register a new MCP server
    Add {
        /// Server name (unique identifier)
        #[arg(short, long)]
        name: String,

        /// Command to run the MCP server
        #[arg(short, long)]
        command: String,

        /// Arguments for the command
        #[arg(short, long)]
        args: Option<String>,

        /// Environment variables (KEY=VALUE, comma-separated)
        #[arg(short, long)]
        env: Option<String>,

        /// Transport type (stdio or sse)
        #[arg(short, long, default_value = "stdio")]
        transport: String,

        /// Description of the server
        #[arg(short, long)]
        description: Option<String>,

        /// Skip the connection test after registering
        #[arg(long)]
        no_test: bool,
    },

    /// Enable an MCP server for a provider
    Enable {
        /// Server name
        name: String,

        /// Target provider
        #[arg(short, long)]
        provider: String,

        /// Target scope
        #[arg(short, long, value_enum)]
        scope: Option<ScopeArg>,
    },

    /// Disable an MCP server for a provider
    Disable {
        /// Server name
        name: String,

        /// Target provider
        #[arg(short, long)]
        provider: String,

        /// Target scope
        #[arg(short, long, value_enum)]
        scope: Option<ScopeArg>,
    },

    /// List all registered MCP servers
    List {
        /// Filter by enabled provider
        #[arg(short, long)]
        provider: Option<String>,
    },

    /// Test an MCP server connection
    Test {
        /// Server name
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum TelemetryCommands {
    /// Enable telemetry collection
    Enable,

    /// Disable telemetry collection
    Disable,

    /// Show telemetry status
    Status,

    /// Export telemetry data
    Export {
        /// Output format
        #[arg(long, value_enum, default_value = "json")]
        format: ExportFormat,

        /// Output file (default: stdout)
        #[arg(long)]
        output: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ContextCommands {
    /// Switch to a context and apply its defaults
    Switch {
        /// Context name
        name: String,
        /// Only show what would change
        #[arg(long)]
        dry_run: bool,
    },

    /// List all configured contexts
    List,

    /// Create a new context
    Create {
        /// Context name
        name: String,
        /// Display name
        #[arg(short, long)]
        display_name: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum DebugCommands {
    /// List active and recent tracked tasks
    Tasks,

    /// Detect hung tasks (running longer than 30 seconds)
    Hangs,

    /// Dump current trace span tree (requires observability feature)
    Trace,
}

#[derive(Subcommand, Debug)]
pub enum VaultCommands {
    /// Initialize a vault repo with .agk/vault.toml and standard asset folders
    Init {
        /// Vault name (defaults to folder name)
        #[arg(short, long)]
        name: Option<String>,

        /// Only show what would change, without modifying anything
        #[arg(long)]
        dry_run: bool,
    },

    /// Attach a local vault directory to this workspace's global config
    Attach {
        /// Path to the vault directory (must contain .agk/vault.toml)
        path: String,

        /// Override the vault ID (defaults to the name in vault.toml)
        #[arg(short, long)]
        id: Option<String>,
    },
}
